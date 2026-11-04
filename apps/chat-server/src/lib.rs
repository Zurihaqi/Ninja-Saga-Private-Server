//! Ninja Saga realtime Chat Server (`/global-chat` and `/clan-chat` Socket.IO namespaces,
//! `/admin/announce` broadcast webhook, `/crossdomain.xml`, and TCP port 843 Flash policy).

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use persistence::{MySqlPool, Row, sqlx};
use serde::Deserialize;
use serde_json::{Value as JsonValue, json};
use socket_protocol::{
    EngineKind, EnginePacket, Limits, SocketPacket, decode_payload, decode_payload_v4,
    decode_socket_lenient, encode_payload, encode_payload_v4, encode_socket,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::Notify,
};

const MAX_BODY: usize = 64 * 1024;
const MAX_PACKET: usize = 32 * 1024;
const MAX_QUEUE: usize = 128;
const MAX_SESSIONS: usize = 1024;
const MAX_HISTORY: usize = 50;
const MAX_MSG_LENGTH: usize = 500;
const DEFAULT_RATE_LIMIT_MS: u64 = 3000;

pub const CROSS_DOMAIN_POLICY: &str = concat!(
    "<?xml version=\"1.0\"?>",
    "<!DOCTYPE cross-domain-policy SYSTEM \"http://www.macromedia.com/xml/dtds/cross-domain-policy.dtd\">",
    "<cross-domain-policy>",
    "<site-control permitted-cross-domain-policies=\"all\"/>",
    "<allow-access-from domain=\"*\" to-ports=\"*\" secure=\"false\"/>",
    "<allow-http-request-headers-from domain=\"*\" headers=\"*\"/>",
    "</cross-domain-policy>"
);

#[derive(Clone)]
pub struct ChatConfig {
    pub admin_secret: String,
    pub max_history: usize,
    pub max_msg_length: usize,
    pub rate_limit_ms: u64,
    pub session_ttl: Duration,
    pub poll_wait: Duration,
    pub capacity: usize,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            admin_secret: std::env::var("CHAT_ADMIN_SECRET").unwrap_or_default(),
            max_history: MAX_HISTORY,
            max_msg_length: MAX_MSG_LENGTH,
            rate_limit_ms: DEFAULT_RATE_LIMIT_MS,
            session_ttl: Duration::from_secs(60),
            poll_wait: Duration::from_secs(20),
            capacity: MAX_SESSIONS,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AuthenticatedCharacter {
    pub id: u64,
    pub payload: JsonValue,
    pub clan_id: Option<u64>,
}

#[derive(Clone)]
pub struct ChatServer {
    inner: Arc<ChatInner>,
}

struct ChatInner {
    pool: Option<MySqlPool>,
    config: ChatConfig,
    started_at: Instant,
    state: Mutex<ChatState>,
}

#[derive(Default)]
struct ChatState {
    sessions: HashMap<String, Session>,
    history: HashMap<String, VecDeque<JsonValue>>,
    mock_characters: HashMap<u64, (String, AuthenticatedCharacter)>,
}

struct NamespaceClient {
    character: Option<AuthenticatedCharacter>,
    last_message_at: Option<Instant>,
}

struct Session {
    eio: u8,
    last_ping: Instant,
    polling: bool,
    queue: VecDeque<EnginePacket>,
    notify: Arc<Notify>,
    namespaces: HashMap<String, NamespaceClient>,
}

#[derive(Deserialize)]
struct PollParams {
    #[serde(rename = "EIO")]
    eio: Option<String>,
    transport: Option<String>,
    sid: Option<String>,
}

#[derive(Deserialize)]
struct AnnounceRequest {
    message: Option<String>,
    channel: Option<String>,
}

fn limits() -> Limits {
    Limits {
        max_payload_bytes: MAX_BODY,
        max_packet_bytes: MAX_PACKET,
        max_packets: MAX_QUEUE,
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn engine_packet(kind: EngineKind, data: impl Into<String>) -> EnginePacket {
    EnginePacket {
        kind,
        data: data.into(),
    }
}

fn socket_event_packet(namespace: &str, name: &str, args: Vec<JsonValue>) -> Option<EnginePacket> {
    let pkt = SocketPacket::Event {
        namespace: namespace.to_owned(),
        name: name.to_owned(),
        args,
    };
    encode_socket(&pkt, limits())
        .ok()
        .map(|encoded| engine_packet(EngineKind::Message, encoded))
}

fn cors_text(status: StatusCode, content_type: &'static str, body: String) -> Response {
    (
        status,
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-store"),
            (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"),
            (header::ACCESS_CONTROL_ALLOW_METHODS, "GET, POST, OPTIONS"),
            (
                header::ACCESS_CONTROL_ALLOW_HEADERS,
                "Content-Type, Authorization, X-Admin-Secret",
            ),
        ],
        body,
    )
        .into_response()
}

fn cors_json(status: StatusCode, body: JsonValue) -> Response {
    cors_text(status, "application/json", body.to_string())
}

fn encode_polling(eio: u8, packets: &[EnginePacket]) -> Response {
    let encoded = if eio == 4 {
        encode_payload_v4(packets, limits())
    } else {
        encode_payload(packets, limits())
    };
    match encoded {
        Ok(text) => cors_text(StatusCode::OK, "text/plain; charset=UTF-8", text),
        Err(_) => cors_json(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": "response framing failed"}),
        ),
    }
}

fn decode_polling(eio: u8, body: &str) -> Result<Vec<EnginePacket>, socket_protocol::Error> {
    if eio == 4 {
        decode_payload_v4(body, limits())
    } else {
        decode_payload(body, limits())
    }
}

impl ChatServer {
    pub fn new(pool: Option<MySqlPool>, config: ChatConfig) -> Self {
        let mut history = HashMap::new();
        history.insert("global".to_owned(), VecDeque::new());
        Self {
            inner: Arc::new(ChatInner {
                pool,
                config,
                started_at: Instant::now(),
                state: Mutex::new(ChatState {
                    sessions: HashMap::new(),
                    history,
                    mock_characters: HashMap::new(),
                }),
            }),
        }
    }

    pub async fn ensure_schema(&self) -> Result<(), sqlx::Error> {
        let Some(pool) = &self.inner.pool else {
            return Ok(());
        };
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS chat_messages (
                id             BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
                channel        VARCHAR(64)     NOT NULL,
                character_id   INT UNSIGNED    NOT NULL,
                character_name VARCHAR(64)     NOT NULL,
                character_data TEXT            NOT NULL,
                message        VARCHAR(500)    NOT NULL,
                timestamp      BIGINT          NOT NULL,
                INDEX idx_channel_ts (channel, timestamp)
            ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4",
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    #[cfg(test)]
    pub fn insert_mock_character(&self, session_key: &str, character: AuthenticatedCharacter) {
        let mut state = self.inner.state.lock().expect("chat state lock poisoned");
        state
            .mock_characters
            .insert(character.id, (session_key.to_owned(), character));
    }

    fn prune_locked(&self, state: &mut ChatState) {
        let ttl = self.inner.config.session_ttl;
        let expired: Vec<String> = state
            .sessions
            .iter()
            .filter(|(_, s)| s.last_ping.elapsed() >= ttl)
            .map(|(sid, _)| sid.clone())
            .collect();
        if expired.is_empty() {
            return;
        }
        let mut affected_global = false;
        let mut affected_clans = HashSet::new();
        for sid in expired {
            if let Some(session) = state.sessions.remove(&sid) {
                session.notify.notify_waiters();
                for (ns, client) in session.namespaces {
                    if let Some(ch) = client.character {
                        if ns == "/global-chat" {
                            affected_global = true;
                        } else if ns == "/clan-chat"
                            && let Some(clan_id) = ch.clan_id
                        {
                            affected_clans.insert(clan_id);
                        }
                    }
                }
            }
        }
        if affected_global {
            Self::broadcast_online_users_locked(state, "/global-chat", None);
        }
        for clan_id in affected_clans {
            Self::broadcast_online_users_locked(state, "/clan-chat", Some(clan_id));
        }
    }

    fn online_users_locked(
        state: &ChatState,
        namespace: &str,
        clan_id: Option<u64>,
    ) -> Vec<JsonValue> {
        let mut list = Vec::new();
        let mut seen = HashSet::new();
        for session in state.sessions.values() {
            if let Some(client) = session.namespaces.get(namespace)
                && let Some(ch) = &client.character
                && (namespace != "/clan-chat" || ch.clan_id == clan_id)
                && seen.insert(ch.id)
            {
                list.push(ch.payload.clone());
            }
        }
        list
    }

    fn push_to_session(session: &mut Session, pkt: EnginePacket) {
        if session.queue.len() < MAX_QUEUE {
            session.queue.push_back(pkt);
            session.notify.notify_waiters();
        }
    }

    fn broadcast_locked(
        state: &mut ChatState,
        namespace: &str,
        clan_id: Option<u64>,
        event: &str,
        payload: JsonValue,
    ) {
        let Some(pkt) = socket_event_packet(namespace, event, vec![payload]) else {
            return;
        };
        for session in state.sessions.values_mut() {
            let should_send = session
                .namespaces
                .get(namespace)
                .and_then(|c| c.character.as_ref())
                .is_some_and(|ch| namespace != "/clan-chat" || ch.clan_id == clan_id);
            if should_send {
                Self::push_to_session(session, pkt.clone());
            }
        }
    }

    fn broadcast_online_users_locked(state: &mut ChatState, namespace: &str, clan_id: Option<u64>) {
        let users = Self::online_users_locked(state, namespace, clan_id);
        Self::broadcast_locked(state, namespace, clan_id, "online-users", json!(users));
    }

    async fn load_history(&self, channel: &str) -> Vec<JsonValue> {
        {
            let state = self.inner.state.lock().expect("chat state lock poisoned");
            if let Some(cached) = state.history.get(channel) {
                return cached.iter().cloned().collect();
            }
        }
        let mut loaded = VecDeque::new();
        if let Some(pool) = &self.inner.pool {
            let limit = self.inner.config.max_history as i64;
            if let Ok(rows) = sqlx::query(
                "SELECT character_data, message, timestamp
                 FROM chat_messages
                 WHERE channel = ?
                 ORDER BY timestamp DESC
                 LIMIT ?",
            )
            .bind(channel)
            .bind(limit)
            .fetch_all(pool)
            .await
            {
                for row in rows.into_iter().rev() {
                    let char_str: String = row.try_get("character_data").unwrap_or_default();
                    let message: String = row.try_get("message").unwrap_or_default();
                    let timestamp: i64 = row.try_get("timestamp").unwrap_or(0);
                    if let Ok(character) = serde_json::from_str::<JsonValue>(&char_str) {
                        loaded.push_back(json!({
                            "character": character,
                            "message": message,
                            "timestamp": timestamp,
                        }));
                    }
                }
            }
        }
        let mut state = self.inner.state.lock().expect("chat state lock poisoned");
        let entry = state
            .history
            .entry(channel.to_owned())
            .or_insert_with(|| loaded.clone());
        entry.iter().cloned().collect()
    }

    async fn save_message(&self, channel: &str, entry: JsonValue) {
        {
            let mut state = self.inner.state.lock().expect("chat state lock poisoned");
            let buf = state.history.entry(channel.to_owned()).or_default();
            buf.push_back(entry.clone());
            while buf.len() > self.inner.config.max_history {
                buf.pop_front();
            }
        }
        if let Some(pool) = &self.inner.pool {
            let char_val = entry.get("character").cloned().unwrap_or_else(|| json!({}));
            let char_id = char_val.get("id").and_then(JsonValue::as_u64).unwrap_or(0) as u32;
            let char_name = char_val
                .get("name")
                .and_then(JsonValue::as_str)
                .unwrap_or("[System]")
                .to_owned();
            let char_data = char_val.to_string();
            let msg = entry
                .get("message")
                .and_then(JsonValue::as_str)
                .unwrap_or("")
                .to_owned();
            let ts = entry
                .get("timestamp")
                .and_then(JsonValue::as_i64)
                .unwrap_or_else(now_ms);
            let _ = sqlx::query(
                "INSERT INTO chat_messages (channel, character_id, character_name, character_data, message, timestamp)
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(channel)
            .bind(char_id)
            .bind(char_name)
            .bind(char_data)
            .bind(msg)
            .bind(ts)
            .execute(pool)
            .await;
        }
    }

    async fn verify_character(
        &self,
        character_id: u64,
        session_key: &str,
        need_clan: bool,
    ) -> Result<AuthenticatedCharacter, &'static str> {
        if character_id == 0 || session_key.trim().is_empty() {
            return Err("Authentication failed.");
        }
        {
            let state = self.inner.state.lock().expect("chat state lock poisoned");
            if let Some((stored_key, mock_char)) = state.mock_characters.get(&character_id) {
                if authentication::matches_session(1, stored_key, session_key)
                    || stored_key == session_key
                {
                    if need_clan && mock_char.clan_id.is_none() {
                        return Err("You are not in a clan.");
                    }
                    return Ok(mock_char.clone());
                }
                return Err("Authentication failed.");
            }
        }
        let Some(pool) = &self.inner.pool else {
            return Err("Authentication failed.");
        };
        let row = sqlx::query(
            "SELECT c.id, c.name, c.level, c.`rank`,
                    c.hair_style, c.hair_color, c.skin_color, c.gender,
                    c.equipment_clothing, c.equipment_back, c.equipment_accessory, c.equipment_weapon,
                    u.id AS user_id, u.session_key, u.account_type
             FROM characters c
             JOIN users u ON u.id = c.user_id
             WHERE c.id = ?
             LIMIT 1",
        )
        .bind(character_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| "Authentication error.")?
        .ok_or("Authentication failed.")?;

        let user_id: u64 = row.try_get("user_id").unwrap_or(0);
        let stored_key: String = row
            .try_get::<Option<String>, _>("session_key")
            .ok()
            .flatten()
            .unwrap_or_default();
        if !authentication::matches_session(user_id, &stored_key, session_key) {
            return Err("Authentication failed.");
        }

        let clan_id = if need_clan {
            let clan_row =
                sqlx::query("SELECT clan_id FROM clan_members WHERE character_id = ? LIMIT 1")
                    .bind(character_id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|_| "Authentication error.")?;
            let Some(clan_row) = clan_row else {
                return Err("You are not in a clan.");
            };
            Some(clan_row.try_get::<u64, _>("clan_id").unwrap_or(0))
        } else {
            None
        };

        let gender: i8 = row.try_get("gender").unwrap_or(0);
        let suffix = if gender == 0 { "m" } else { "f" };
        let raw_hair: Option<String> = row.try_get("hair_style").ok().flatten();
        let hair_style = match raw_hair.as_deref() {
            None | Some("") => format!("hair_01_{suffix}"),
            Some(val) => {
                if let Ok(num) = val.parse::<u32>() {
                    format!("hair_{num:02}_{suffix}")
                } else {
                    val.to_owned()
                }
            }
        };
        let hair_color = row
            .try_get::<Option<String>, _>("hair_color")
            .ok()
            .flatten()
            .unwrap_or_else(|| "0|0".to_owned());
        let skin_color = row
            .try_get::<Option<String>, _>("skin_color")
            .ok()
            .flatten()
            .unwrap_or_else(|| "null".to_owned());
        let rank = row
            .try_get::<i32, _>("rank")
            .or_else(|_| row.try_get::<i8, _>("rank").map(i32::from))
            .unwrap_or(1);
        let account_type = row
            .try_get::<i32, _>("account_type")
            .or_else(|_| row.try_get::<i8, _>("account_type").map(i32::from))
            .unwrap_or(0);
        let payload = json!({
            "id": character_id,
            "name": row.try_get::<String, _>("name").unwrap_or_default(),
            "level": row.try_get::<i32, _>("level").unwrap_or(1),
            "rank": rank,
            "account_type": account_type,
            "character_sets": {
                "hair_style": hair_style,
                "hair_color": hair_color,
                "skin_color": skin_color,
                "face": format!("face_01_{suffix}"),
                "clothing": row.try_get::<Option<String>, _>("equipment_clothing").ok().flatten(),
                "back_item": row.try_get::<Option<String>, _>("equipment_back").ok().flatten(),
                "accessory": row.try_get::<Option<String>, _>("equipment_accessory").ok().flatten(),
                "weapon": row.try_get::<Option<String>, _>("equipment_weapon").ok().flatten(),
            }
        });

        Ok(AuthenticatedCharacter {
            id: character_id,
            payload,
            clan_id,
        })
    }
}

pub fn router(server: ChatServer) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/crossdomain.xml", get(crossdomain_handler))
        .route(
            "/admin/announce",
            post(announce_handler).options(options_handler),
        )
        .route(
            "/socket.io/",
            get(socket_poll_handler)
                .post(socket_post_handler)
                .options(options_handler),
        )
        .route(
            "/socket.io",
            get(socket_poll_handler)
                .post(socket_post_handler)
                .options(options_handler),
        )
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .with_state(server)
}

async fn options_handler() -> Response {
    cors_text(StatusCode::NO_CONTENT, "text/plain", String::new())
}

async fn health_handler(State(server): State<ChatServer>) -> Response {
    let uptime = server.inner.started_at.elapsed().as_secs_f64();
    cors_json(StatusCode::OK, json!({ "status": "ok", "uptime": uptime }))
}

async fn crossdomain_handler() -> Response {
    cors_text(
        StatusCode::OK,
        "text/x-cross-domain-policy",
        CROSS_DOMAIN_POLICY.to_owned(),
    )
}

async fn announce_handler(
    State(server): State<ChatServer>,
    headers: HeaderMap,
    Json(body): Json<AnnounceRequest>,
) -> Response {
    let expected = &server.inner.config.admin_secret;
    let supplied = headers
        .get("x-admin-secret")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if expected.is_empty() || supplied != expected {
        return cors_json(StatusCode::UNAUTHORIZED, json!({ "error": "Unauthorized" }));
    }
    let text: String = body
        .message
        .unwrap_or_default()
        .trim()
        .chars()
        .take(server.inner.config.max_msg_length)
        .collect();
    if text.is_empty() {
        return cors_json(
            StatusCode::BAD_REQUEST,
            json!({ "error": "Message cannot be empty" }),
        );
    }
    let target_channel = body
        .channel
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("global")
        .to_owned();
    let ts = now_ms();
    let sys_payload = json!({
        "character": {
            "id": 0,
            "name": "[System]",
            "level": 99,
            "rank": 10,
            "account_type": 99,
            "character_sets": { "hair_style": 1 }
        },
        "message": text,
        "timestamp": ts
    });
    let announce_payload = json!({
        "message": text,
        "timestamp": ts
    });

    server
        .save_message(&target_channel, sys_payload.clone())
        .await;

    {
        let mut state = server.inner.state.lock().expect("chat state lock poisoned");
        if let Some(clan_str) = target_channel.strip_prefix("clan:") {
            if let Ok(clan_id) = clan_str.parse::<u64>() {
                ChatServer::broadcast_locked(
                    &mut state,
                    "/clan-chat",
                    Some(clan_id),
                    "announcement",
                    announce_payload,
                );
                ChatServer::broadcast_locked(
                    &mut state,
                    "/clan-chat",
                    Some(clan_id),
                    "message",
                    sys_payload,
                );
            }
        } else {
            ChatServer::broadcast_locked(
                &mut state,
                "/global-chat",
                None,
                "announcement",
                announce_payload,
            );
            ChatServer::broadcast_locked(&mut state, "/global-chat", None, "message", sys_payload);
        }
    }

    cors_json(StatusCode::OK, json!({ "ok": true }))
}

fn parse_eio(query: &PollParams) -> Result<u8, &'static str> {
    let transport = query.transport.as_deref().unwrap_or("polling");
    if transport != "polling" {
        return Err("only transport=polling is supported");
    }
    match query.eio.as_deref().unwrap_or("3") {
        "3" => Ok(3),
        "4" => Ok(4),
        _ => Err("only EIO=3 and EIO=4 are supported"),
    }
}

struct PollGuard {
    server: ChatServer,
    sid: String,
}

impl Drop for PollGuard {
    fn drop(&mut self) {
        if let Ok(mut state) = self.server.inner.state.lock()
            && let Some(session) = state.sessions.get_mut(&self.sid)
        {
            session.polling = false;
        }
    }
}

async fn socket_poll_handler(
    State(server): State<ChatServer>,
    Query(query): Query<PollParams>,
) -> Response {
    let eio = match parse_eio(&query) {
        Ok(v) => v,
        Err(msg) => return cors_json(StatusCode::BAD_REQUEST, json!({ "error": msg })),
    };

    let Some(sid) = query.sid else {
        let mut state = server.inner.state.lock().expect("chat state lock poisoned");
        server.prune_locked(&mut state);
        if state.sessions.len() >= server.inner.config.capacity {
            return cors_json(
                StatusCode::SERVICE_UNAVAILABLE,
                json!({ "error": "session capacity reached" }),
            );
        }
        let sid = loop {
            let mut bytes = [0u8; 16];
            if getrandom::fill(&mut bytes).is_err() {
                return cors_json(
                    StatusCode::SERVICE_UNAVAILABLE,
                    json!({ "error": "session entropy unavailable" }),
                );
            }
            let id: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
            if !state.sessions.contains_key(&id) {
                break id;
            }
        };
        let mut initial_queue = VecDeque::new();
        if eio == 4 {
            initial_queue.push_back(engine_packet(EngineKind::Message, "0"));
        }
        state.sessions.insert(
            sid.clone(),
            Session {
                eio,
                last_ping: Instant::now(),
                polling: false,
                queue: initial_queue,
                notify: Arc::new(Notify::new()),
                namespaces: HashMap::new(),
            },
        );
        return encode_polling(
            eio,
            &[engine_packet(
                EngineKind::Open,
                json!({
                    "sid": sid,
                    "upgrades": [],
                    "pingInterval": 25000,
                    "pingTimeout": 20000,
                    "maxPayload": MAX_BODY
                })
                .to_string(),
            )],
        );
    };

    let (notify, session_eio) = {
        let mut state = server.inner.state.lock().expect("chat state lock poisoned");
        server.prune_locked(&mut state);
        let Some(session) = state.sessions.get_mut(&sid) else {
            return cors_json(
                StatusCode::BAD_REQUEST,
                json!({ "error": "unknown or expired session" }),
            );
        };
        session.last_ping = Instant::now();
        if !session.queue.is_empty() {
            let packets: Vec<_> = session.queue.drain(..).collect();
            return encode_polling(session.eio, &packets);
        }
        if session.polling {
            return cors_json(
                StatusCode::BAD_REQUEST,
                json!({ "error": "concurrent polling is unsupported" }),
            );
        }
        session.polling = true;
        (session.notify.clone(), session.eio)
    };

    let _guard = PollGuard {
        server: server.clone(),
        sid: sid.clone(),
    };
    let deadline = tokio::time::Instant::now() + server.inner.config.poll_wait;
    loop {
        let notified = notify.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        {
            let mut state = server.inner.state.lock().expect("chat state lock poisoned");
            server.prune_locked(&mut state);
            let Some(session) = state.sessions.get_mut(&sid) else {
                return cors_json(
                    StatusCode::BAD_REQUEST,
                    json!({ "error": "unknown or expired session" }),
                );
            };
            if !session.queue.is_empty() {
                let packets: Vec<_> = session.queue.drain(..).collect();
                return encode_polling(session.eio, &packets);
            }
        }
        if tokio::time::timeout_at(deadline, notified).await.is_err() {
            return encode_polling(session_eio, &[engine_packet(EngineKind::Noop, "")]);
        }
    }
}

async fn socket_post_handler(
    State(server): State<ChatServer>,
    Query(query): Query<PollParams>,
    body: String,
) -> Response {
    let eio = match parse_eio(&query) {
        Ok(v) => v,
        Err(msg) => return cors_json(StatusCode::BAD_REQUEST, json!({ "error": msg })),
    };
    let Some(sid) = query.sid else {
        return cors_json(
            StatusCode::BAD_REQUEST,
            json!({ "error": "POST requires a session ID" }),
        );
    };
    let session_eio = {
        let mut state = server.inner.state.lock().expect("chat state lock poisoned");
        server.prune_locked(&mut state);
        let Some(session) = state.sessions.get_mut(&sid) else {
            return cors_json(
                StatusCode::BAD_REQUEST,
                json!({ "error": "unknown or expired session" }),
            );
        };
        session.last_ping = Instant::now();
        session.eio
    };
    let packets = match decode_polling(session_eio.max(eio), &body)
        .or_else(|_| decode_polling(3, &body))
        .or_else(|_| decode_polling(4, &body))
    {
        Ok(pkts) if !pkts.is_empty() => pkts,
        _ => {
            return cors_json(
                StatusCode::BAD_REQUEST,
                json!({ "error": "malformed polling payload" }),
            );
        }
    };

    for incoming in packets {
        match incoming.kind {
            EngineKind::Ping => {
                let mut state = server.inner.state.lock().expect("chat state lock poisoned");
                if let Some(session) = state.sessions.get_mut(&sid) {
                    session.last_ping = Instant::now();
                    ChatServer::push_to_session(
                        session,
                        engine_packet(EngineKind::Pong, incoming.data),
                    );
                }
            }
            EngineKind::Pong => {
                let mut state = server.inner.state.lock().expect("chat state lock poisoned");
                if let Some(session) = state.sessions.get_mut(&sid) {
                    session.last_ping = Instant::now();
                }
            }
            EngineKind::Close => {
                let mut state = server.inner.state.lock().expect("chat state lock poisoned");
                if let Some(session) = state.sessions.remove(&sid) {
                    session.notify.notify_waiters();
                    let mut refresh_global = false;
                    let mut refresh_clans = HashSet::new();
                    for (ns, client) in session.namespaces {
                        if let Some(ch) = client.character {
                            if ns == "/global-chat" {
                                refresh_global = true;
                            } else if ns == "/clan-chat"
                                && let Some(cid) = ch.clan_id
                            {
                                refresh_clans.insert(cid);
                            }
                        }
                    }
                    if refresh_global {
                        ChatServer::broadcast_online_users_locked(&mut state, "/global-chat", None);
                    }
                    for cid in refresh_clans {
                        ChatServer::broadcast_online_users_locked(
                            &mut state,
                            "/clan-chat",
                            Some(cid),
                        );
                    }
                }
                return cors_text(StatusCode::OK, "text/plain; charset=UTF-8", "ok".to_owned());
            }
            EngineKind::Message => {
                let Ok(socket_pkt) = decode_socket_lenient(&incoming.data, limits()) else {
                    return cors_json(
                        StatusCode::BAD_REQUEST,
                        json!({ "error": "unsupported Socket.IO frame" }),
                    );
                };
                handle_socket_packet(&server, &sid, socket_pkt).await;
            }
            _ => {}
        }
    }

    cors_text(StatusCode::OK, "text/plain; charset=UTF-8", "ok".to_owned())
}

async fn handle_socket_packet(server: &ChatServer, sid: &str, packet: SocketPacket) {
    match packet {
        SocketPacket::Connect { namespace } => {
            if !matches!(namespace.as_str(), "/" | "/global-chat" | "/clan-chat") {
                return;
            }
            let mut state = server.inner.state.lock().expect("chat state lock poisoned");
            let Some(session) = state.sessions.get_mut(sid) else {
                return;
            };
            session
                .namespaces
                .entry(namespace.clone())
                .or_insert(NamespaceClient {
                    character: None,
                    last_message_at: None,
                });
            if let Ok(encoded) = encode_socket(&SocketPacket::Connect { namespace }, limits()) {
                ChatServer::push_to_session(session, engine_packet(EngineKind::Message, encoded));
            }
        }
        SocketPacket::Disconnect { namespace } => {
            let mut state = server.inner.state.lock().expect("chat state lock poisoned");
            let removed = state
                .sessions
                .get_mut(sid)
                .and_then(|s| s.namespaces.remove(&namespace))
                .and_then(|c| c.character);
            if let Some(ch) = removed {
                if namespace == "/global-chat" {
                    ChatServer::broadcast_online_users_locked(&mut state, "/global-chat", None);
                } else if namespace == "/clan-chat"
                    && let Some(clan_id) = ch.clan_id
                {
                    ChatServer::broadcast_online_users_locked(
                        &mut state,
                        "/clan-chat",
                        Some(clan_id),
                    );
                }
            }
        }
        SocketPacket::Event {
            namespace,
            name,
            args,
        } => {
            if !matches!(namespace.as_str(), "/global-chat" | "/clan-chat") {
                return;
            }
            match name.as_str() {
                "auth" => handle_auth_event(server, sid, &namespace, &args).await,
                "open-chatbox" => handle_open_chatbox_event(server, sid, &namespace).await,
                "sendMessage" => handle_send_message_event(server, sid, &namespace, &args).await,
                _ => {}
            }
        }
    }
}

fn parse_auth_args(args: &[JsonValue]) -> (u64, String) {
    if let Some(first) = args.first()
        && let Some(obj) = first.as_object()
    {
        let char_id = obj
            .get("character_id")
            .or_else(|| obj.get("id"))
            .and_then(|v| {
                v.as_u64()
                    .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
            })
            .unwrap_or(0);
        let session_key = obj
            .get("session_key")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        return (char_id, session_key);
    }
    let char_id = args
        .first()
        .and_then(|v| {
            v.as_u64()
                .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
        })
        .unwrap_or(0);
    let session_key = args
        .get(1)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();
    (char_id, session_key)
}

async fn handle_auth_event(server: &ChatServer, sid: &str, namespace: &str, args: &[JsonValue]) {
    let (character_id, session_key) = parse_auth_args(args);
    let need_clan = namespace == "/clan-chat";
    let auth_result = server
        .verify_character(character_id, &session_key, need_clan)
        .await;

    let character = match auth_result {
        Ok(ch) => ch,
        Err(err_msg) => {
            let mut state = server.inner.state.lock().expect("chat state lock poisoned");
            if let Some(session) = state.sessions.get_mut(sid) {
                if let Some(pkt) = socket_event_packet(namespace, "error", vec![json!(err_msg)]) {
                    ChatServer::push_to_session(session, pkt);
                }
                if let Ok(disc) = encode_socket(
                    &SocketPacket::Disconnect {
                        namespace: namespace.to_owned(),
                    },
                    limits(),
                ) {
                    ChatServer::push_to_session(session, engine_packet(EngineKind::Message, disc));
                }
                session.namespaces.remove(namespace);
            }
            return;
        }
    };

    let channel_key = if let Some(clan_id) = character.clan_id {
        format!("clan:{clan_id}")
    } else {
        "global".to_owned()
    };
    let history = server.load_history(&channel_key).await;

    let mut state = server.inner.state.lock().expect("chat state lock poisoned");
    // Evict any previous session in the same namespace with the same character_id
    for (other_sid, other_session) in state.sessions.iter_mut() {
        if other_sid != sid
            && let Some(other_client) = other_session.namespaces.get(namespace)
            && other_client
                .character
                .as_ref()
                .is_some_and(|c| c.id == character.id)
        {
            other_session.namespaces.remove(namespace);
            if let Ok(disc) = encode_socket(
                &SocketPacket::Disconnect {
                    namespace: namespace.to_owned(),
                },
                limits(),
            ) {
                ChatServer::push_to_session(
                    other_session,
                    engine_packet(EngineKind::Message, disc),
                );
            }
        }
    }

    let clan_id = character.clan_id;
    if let Some(session) = state.sessions.get_mut(sid) {
        let client = session
            .namespaces
            .entry(namespace.to_owned())
            .or_insert(NamespaceClient {
                character: None,
                last_message_at: None,
            });
        client.character = Some(character);
    }

    let online_users = ChatServer::online_users_locked(&state, namespace, clan_id);
    if let Some(session) = state.sessions.get_mut(sid) {
        if let Some(pkt) = socket_event_packet(namespace, "nickname-colors", vec![json!({})]) {
            ChatServer::push_to_session(session, pkt);
        }
        if let Some(pkt) = socket_event_packet(namespace, "online-users", vec![json!(online_users)])
        {
            ChatServer::push_to_session(session, pkt);
        }
        if let Some(pkt) = socket_event_packet(namespace, "history", vec![json!(history)]) {
            ChatServer::push_to_session(session, pkt);
        }
    }
    ChatServer::broadcast_online_users_locked(&mut state, namespace, clan_id);
}

async fn handle_open_chatbox_event(server: &ChatServer, sid: &str, namespace: &str) {
    let character = {
        let state = server.inner.state.lock().expect("chat state lock poisoned");
        state
            .sessions
            .get(sid)
            .and_then(|s| s.namespaces.get(namespace))
            .and_then(|c| c.character.clone())
    };
    let Some(character) = character else {
        return;
    };
    let channel_key = if let Some(clan_id) = character.clan_id {
        format!("clan:{clan_id}")
    } else {
        "global".to_owned()
    };
    let history = server.load_history(&channel_key).await;

    let mut state = server.inner.state.lock().expect("chat state lock poisoned");
    let online_users = ChatServer::online_users_locked(&state, namespace, character.clan_id);
    if let Some(session) = state.sessions.get_mut(sid) {
        if let Some(pkt) = socket_event_packet(namespace, "online-users", vec![json!(online_users)])
        {
            ChatServer::push_to_session(session, pkt);
        }
        if let Some(pkt) = socket_event_packet(namespace, "history", vec![json!(history)]) {
            ChatServer::push_to_session(session, pkt);
        }
    }
}

async fn handle_send_message_event(
    server: &ChatServer,
    sid: &str,
    namespace: &str,
    args: &[JsonValue],
) {
    let raw_text = args
        .first()
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_owned();
    if raw_text.is_empty() {
        return;
    }

    let character = {
        let mut state = server.inner.state.lock().expect("chat state lock poisoned");
        let Some(session) = state.sessions.get_mut(sid) else {
            return;
        };
        let Some(client) = session.namespaces.get_mut(namespace) else {
            return;
        };
        let Some(character) = client.character.clone() else {
            return;
        };
        if let Some(last) = client.last_message_at
            && last.elapsed() < Duration::from_millis(server.inner.config.rate_limit_ms)
        {
            if let Some(pkt) = socket_event_packet(
                namespace,
                "warning",
                vec![json!("Please wait before sending another message.")],
            ) {
                ChatServer::push_to_session(session, pkt);
            }
            return;
        }
        if raw_text.chars().count() > server.inner.config.max_msg_length {
            if let Some(pkt) = socket_event_packet(
                namespace,
                "warning",
                vec![json!(format!(
                    "Message cannot exceed {} characters.",
                    server.inner.config.max_msg_length
                ))],
            ) {
                ChatServer::push_to_session(session, pkt);
            }
            return;
        }
        client.last_message_at = Some(Instant::now());
        character
    };

    let channel_key = if let Some(clan_id) = character.clan_id {
        format!("clan:{clan_id}")
    } else {
        "global".to_owned()
    };
    let entry = json!({
        "character": character.payload,
        "message": raw_text,
        "timestamp": now_ms(),
    });
    server.save_message(&channel_key, entry.clone()).await;

    let mut state = server.inner.state.lock().expect("chat state lock poisoned");
    ChatServer::broadcast_locked(&mut state, namespace, character.clan_id, "message", entry);
}

pub fn spawn_policy_server(bind_addr: &str) {
    let addr = bind_addr.to_owned();
    tokio::spawn(async move {
        let listener = match TcpListener::bind(&addr).await {
            Ok(l) => {
                tracing::info!(address = %addr, "Flash socket policy server listening");
                l
            }
            Err(err) => {
                tracing::warn!(address = %addr, error = %err, "Flash socket policy server could not bind");
                return;
            }
        };
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 256];
                let _ = tokio::time::timeout(Duration::from_secs(3), socket.read(&mut buf)).await;
                let mut reply = Vec::with_capacity(CROSS_DOMAIN_POLICY.len() + 1);
                reply.extend_from_slice(CROSS_DOMAIN_POLICY.as_bytes());
                reply.push(0);
                let _ = socket.write_all(&reply).await;
                let _ = socket.shutdown().await;
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    fn test_server() -> ChatServer {
        let cfg = ChatConfig {
            admin_secret: "test_secret".to_owned(),
            max_history: 50,
            max_msg_length: 500,
            rate_limit_ms: 50,
            session_ttl: Duration::from_secs(45),
            poll_wait: Duration::from_millis(10),
            capacity: 16,
        };
        let srv = ChatServer::new(None, cfg);
        srv.insert_mock_character(
            "valid_session_1",
            AuthenticatedCharacter {
                id: 101,
                payload: json!({
                    "id": 101,
                    "name": "Naruto",
                    "level": 40,
                    "rank": 3,
                    "account_type": 1,
                    "character_sets": { "hair_style": "hair_01_m" }
                }),
                clan_id: Some(7),
            },
        );
        srv.insert_mock_character(
            "valid_session_2",
            AuthenticatedCharacter {
                id: 102,
                payload: json!({
                    "id": 102,
                    "name": "Sasuke",
                    "level": 40,
                    "rank": 3,
                    "account_type": 0,
                    "character_sets": { "hair_style": "hair_02_m" }
                }),
                clan_id: None,
            },
        );
        srv
    }

    async fn call(
        app: &Router,
        method: &str,
        uri: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> (StatusCode, String) {
        let mut req = Request::builder().method(method).uri(uri);
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        let resp = app
            .clone()
            .oneshot(req.body(Body::from(body.to_owned())).unwrap())
            .await
            .unwrap();
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), MAX_BODY).await.unwrap();
        (status, String::from_utf8(bytes.to_vec()).unwrap())
    }

    async fn open_session(app: &Router) -> String {
        let (status, body) = call(app, "GET", "/socket.io/?EIO=3&transport=polling", &[], "").await;
        assert_eq!(status, StatusCode::OK);
        let pkts = decode_payload(&body, limits()).unwrap();
        let info: JsonValue = serde_json::from_str(&pkts[0].data).unwrap();
        info["sid"].as_str().unwrap().to_owned()
    }

    async fn post_frames(app: &Router, sid: &str, frames: &[EnginePacket]) {
        let uri = format!("/socket.io/?EIO=3&transport=polling&sid={sid}");
        let payload = encode_payload(frames, limits()).unwrap();
        let (status, _) = call(app, "POST", &uri, &[], &payload).await;
        assert_eq!(status, StatusCode::OK);
    }

    async fn poll_events(app: &Router, sid: &str) -> Vec<SocketPacket> {
        let uri = format!("/socket.io/?EIO=3&transport=polling&sid={sid}");
        let (status, body) = call(app, "GET", &uri, &[], "").await;
        assert_eq!(status, StatusCode::OK);
        let pkts = decode_payload(&body, limits()).unwrap();
        pkts.into_iter()
            .filter(|p| p.kind == EngineKind::Message)
            .filter_map(|p| decode_socket_lenient(&p.data, limits()).ok())
            .collect()
    }

    #[tokio::test]
    async fn announcements_require_a_configured_secret() {
        let app = router(ChatServer::new(
            None,
            ChatConfig {
                admin_secret: String::new(),
                ..ChatConfig::default()
            },
        ));
        for secret in ["", "ninjasage_admin_secret"] {
            let (status, _) = call(
                &app,
                "POST",
                "/admin/announce",
                &[
                    ("x-admin-secret", secret),
                    ("content-type", "application/json"),
                ],
                r#"{"message":"unauthorized"}"#,
            )
            .await;
            assert_eq!(status, StatusCode::UNAUTHORIZED);
        }
    }

    #[tokio::test]
    async fn global_and_clan_chat_lifecycle_and_admin_announce() {
        let srv = test_server();
        let app = shared::health_router().merge(router(srv));
        assert_eq!(
            call(&app, "GET", "/health/live", &[], "").await.0,
            StatusCode::OK
        );
        assert_eq!(
            call(&app, "GET", "/health/ready", &[], "").await.0,
            StatusCode::SERVICE_UNAVAILABLE
        );

        let (status, xml) = call(&app, "GET", "/crossdomain.xml", &[], "").await;
        assert_eq!(status, StatusCode::OK);
        assert!(xml.contains("<cross-domain-policy>"));

        let (status, health) = call(&app, "GET", "/health", &[], "").await;
        assert_eq!(status, StatusCode::OK);
        let health_json: JsonValue = serde_json::from_str(&health).unwrap();
        assert_eq!(health_json["status"], "ok");

        let sid1 = open_session(&app).await;
        post_frames(
            &app,
            &sid1,
            &[
                engine_packet(EngineKind::Message, "0/global-chat,"),
                engine_packet(
                    EngineKind::Message,
                    r#"2/global-chat,["auth",{"character_id":101,"session_key":"valid_session_1"}]"#,
                ),
            ],
        )
        .await;
        let events = poll_events(&app, &sid1).await;
        assert!(events.iter().any(
            |e| matches!(e, SocketPacket::Connect { namespace } if namespace == "/global-chat")
        ));
        assert!(
            events.iter().any(
                |e| matches!(e, SocketPacket::Event { name, .. } if name == "nickname-colors")
            )
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, SocketPacket::Event { name, .. } if name == "online-users"))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, SocketPacket::Event { name, .. } if name == "history"))
        );

        // Send message on global chat
        post_frames(
            &app,
            &sid1,
            &[engine_packet(
                EngineKind::Message,
                r#"2/global-chat,["sendMessage","Believe it!"]"#,
            )],
        )
        .await;
        let events = poll_events(&app, &sid1).await;
        assert!(events.iter().any(|e| matches!(
            e,
            SocketPacket::Event { name, args, .. }
            if name == "message" && args[0]["message"] == "Believe it!"
        )));

        // Immediate second message hits rate limit warning
        post_frames(
            &app,
            &sid1,
            &[engine_packet(
                EngineKind::Message,
                r#"2/global-chat,["sendMessage","Spam!"]"#,
            )],
        )
        .await;
        let events = poll_events(&app, &sid1).await;
        assert!(events.iter().any(|e| matches!(
            e,
            SocketPacket::Event { name, .. } if name == "warning"
        )));

        // Admin announce
        let (status, _) = call(
            &app,
            "POST",
            "/admin/announce",
            &[
                ("content-type", "application/json"),
                ("x-admin-secret", "test_secret"),
            ],
            r#"{"message":"Server maintenance soon","channel":"global"}"#,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let events = poll_events(&app, &sid1).await;
        assert!(events.iter().any(|e| matches!(
            e,
            SocketPacket::Event { name, args, .. }
            if name == "announcement" && args[0]["message"] == "Server maintenance soon"
        )));

        // Clan chat: character 102 is not in a clan -> receives error & disconnect
        let sid2 = open_session(&app).await;
        post_frames(
            &app,
            &sid2,
            &[
                engine_packet(EngineKind::Message, "0/clan-chat,"),
                engine_packet(
                    EngineKind::Message,
                    r#"2/clan-chat,["auth",{"character_id":102,"session_key":"valid_session_2"}]"#,
                ),
            ],
        )
        .await;
        let events = poll_events(&app, &sid2).await;
        assert!(events.iter().any(|e| matches!(
            e,
            SocketPacket::Event { name, args, .. }
            if name == "error" && args[0] == "You are not in a clan."
        )));
    }
}
