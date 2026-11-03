//! Opt-in protocol feasibility experiment. No account or gameplay service is exposed.
use axum::{
    Router,
    extract::{DefaultBodyLimit, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
use serde_json::json;
use socket_protocol::{
    EngineKind, EnginePacket, Limits, SocketPacket, decode_payload, decode_socket, encode_payload,
    encode_socket,
};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::Notify;

const MAX_BODY: usize = 32 * 1024;
const MAX_QUEUE: usize = 64;
const MAX_SESSIONS: usize = 128;

#[derive(Clone)]
struct Server {
    sessions: Arc<Mutex<HashMap<String, Session>>>,
    ttl: Duration,
    poll_wait: Duration,
    capacity: usize,
}

struct Session {
    last_ping: Instant,
    connected: bool,
    polling: bool,
    queue: VecDeque<EnginePacket>,
    notify: Arc<Notify>,
}

#[derive(Deserialize)]
struct Parameters {
    #[serde(rename = "EIO")]
    eio: String,
    transport: String,
    sid: Option<String>,
}

fn limits() -> Limits {
    Limits {
        max_payload_bytes: MAX_BODY,
        max_packet_bytes: 8 * 1024,
        max_packets: MAX_QUEUE,
    }
}

fn packet(kind: EngineKind, data: impl Into<String>) -> EnginePacket {
    EnginePacket {
        kind,
        data: data.into(),
    }
}

fn error(status: StatusCode, message: &'static str) -> Response {
    (status, axum::Json(json!({"error": message}))).into_response()
}

fn text(value: String) -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/plain; charset=UTF-8"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        value,
    )
        .into_response()
}

fn payload(packets: &[EnginePacket]) -> Response {
    match encode_payload(packets, limits()) {
        Ok(value) => text(value),
        Err(_) => error(StatusCode::INTERNAL_SERVER_ERROR, "response framing failed"),
    }
}

fn validate(query: &Parameters) -> Result<(), &'static str> {
    if query.eio != "3" || query.transport != "polling" {
        return Err("only EIO=3 transport=polling is supported");
    }
    if query
        .sid
        .as_ref()
        .is_some_and(|sid| sid.len() != 64 || !sid.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return Err("invalid session ID");
    }
    Ok(())
}

pub fn router() -> Router {
    build(Server {
        sessions: Arc::default(),
        ttl: Duration::from_secs(45),
        poll_wait: Duration::from_secs(20),
        capacity: MAX_SESSIONS,
    })
}

fn build(state: Server) -> Router {
    Router::new()
        .route("/socket.io/", get(poll).post(post))
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .with_state(state)
}

impl Server {
    fn prune(&self, sessions: &mut HashMap<String, Session>) {
        sessions.retain(|_, s| s.last_ping.elapsed() < self.ttl);
    }
}

async fn poll(State(server): State<Server>, Query(query): Query<Parameters>) -> Response {
    if let Err(response) = validate(&query) {
        return error(StatusCode::BAD_REQUEST, response);
    }
    let Some(sid) = query.sid else {
        let mut sessions = server.sessions.lock().expect("session lock poisoned");
        server.prune(&mut sessions);
        if sessions.len() >= server.capacity {
            return error(StatusCode::SERVICE_UNAVAILABLE, "session capacity reached");
        }
        let sid = loop {
            let mut bytes = [0u8; 32];
            if getrandom::fill(&mut bytes).is_err() {
                return error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "session entropy unavailable",
                );
            }
            let id: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
            if !sessions.contains_key(&id) {
                break id;
            }
        };
        sessions.insert(
            sid.clone(),
            Session {
                last_ping: Instant::now(),
                connected: false,
                polling: false,
                queue: VecDeque::new(),
                notify: Arc::new(Notify::new()),
            },
        );
        return payload(&[packet(
            EngineKind::Open,
            json!({
                "sid": sid, "upgrades": [], "pingInterval": 25000, "pingTimeout": 20000
            })
            .to_string(),
        )]);
    };
    let notify = {
        let mut sessions = server.sessions.lock().expect("session lock poisoned");
        server.prune(&mut sessions);
        let Some(session) = sessions.get_mut(&sid) else {
            return error(StatusCode::BAD_REQUEST, "unknown or expired session");
        };
        if session.polling {
            return error(StatusCode::BAD_REQUEST, "concurrent polling is unsupported");
        }
        session.polling = true;
        session.notify.clone()
    };
    // Drop also runs if the HTTP request future is cancelled.
    let _guard = PollGuard {
        server: server.clone(),
        sid: sid.clone(),
    };
    let deadline = tokio::time::Instant::now() + server.poll_wait;
    loop {
        let notified = notify.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        {
            let mut sessions = server.sessions.lock().expect("session lock poisoned");
            server.prune(&mut sessions);
            let Some(session) = sessions.get_mut(&sid) else {
                return error(StatusCode::BAD_REQUEST, "unknown or expired session");
            };
            if !session.queue.is_empty() {
                let packets: Vec<_> = session.queue.drain(..).collect();
                return payload(&packets);
            }
        }
        if tokio::time::timeout_at(deadline, notified).await.is_err() {
            return payload(&[packet(EngineKind::Noop, "")]);
        }
    }
}

struct PollGuard {
    server: Server,
    sid: String,
}
impl Drop for PollGuard {
    fn drop(&mut self) {
        if let Some(session) = self
            .server
            .sessions
            .lock()
            .expect("session lock poisoned")
            .get_mut(&self.sid)
        {
            session.polling = false;
        }
    }
}

async fn post(
    State(server): State<Server>,
    Query(query): Query<Parameters>,
    body: String,
) -> Response {
    if let Err(response) = validate(&query) {
        return error(StatusCode::BAD_REQUEST, response);
    }
    let Some(sid) = query.sid else {
        return error(StatusCode::BAD_REQUEST, "POST requires a session ID");
    };
    let Ok(packets) = decode_payload(&body, limits()) else {
        return error(StatusCode::BAD_REQUEST, "malformed polling payload");
    };
    if packets.is_empty() {
        return error(StatusCode::BAD_REQUEST, "empty polling payload");
    }
    let mut sessions = server.sessions.lock().expect("session lock poisoned");
    server.prune(&mut sessions);
    let Some(session) = sessions.get_mut(&sid) else {
        return error(StatusCode::BAD_REQUEST, "unknown or expired session");
    };
    // Stage a complete request before mutating state, including all bounds checks.
    let mut connected = session.connected;
    let mut replies = Vec::new();
    let mut ping = false;
    let mut close = false;
    for incoming in packets {
        if close {
            return error(
                StatusCode::BAD_REQUEST,
                "packets after close are unsupported",
            );
        }
        match incoming.kind {
            EngineKind::Ping => {
                ping = true;
                replies.push(packet(EngineKind::Pong, incoming.data));
            }
            EngineKind::Close if incoming.data.is_empty() => {
                close = true;
            }
            EngineKind::Message => {
                let Ok(socket) = decode_socket(&incoming.data, limits()) else {
                    return error(StatusCode::BAD_REQUEST, "unsupported Socket.IO frame");
                };
                let response = match socket {
                    SocketPacket::Connect { namespace } if namespace == "/compat" && !connected => {
                        connected = true;
                        SocketPacket::Connect { namespace }
                    }
                    SocketPacket::Disconnect { namespace }
                        if namespace == "/compat" && connected =>
                    {
                        connected = false;
                        SocketPacket::Disconnect { namespace }
                    }
                    SocketPacket::Event {
                        namespace,
                        name,
                        args,
                    } if namespace == "/compat" && name == "echo" && connected => {
                        SocketPacket::Event {
                            namespace,
                            name,
                            args,
                        }
                    }
                    _ => {
                        return error(
                            StatusCode::BAD_REQUEST,
                            "only connected /compat echo is supported",
                        );
                    }
                };
                let Ok(encoded) = encode_socket(&response, limits()) else {
                    return error(StatusCode::BAD_REQUEST, "response exceeds frame limit");
                };
                replies.push(packet(EngineKind::Message, encoded));
            }
            _ => return error(StatusCode::BAD_REQUEST, "unsupported Engine.IO packet"),
        }
    }
    if session.queue.len() + replies.len() > MAX_QUEUE {
        return error(StatusCode::TOO_MANY_REQUESTS, "session queue full");
    }
    // Aggregate bytes must fit a single outgoing polling payload, not just packet count.
    let combined: Vec<_> = session
        .queue
        .iter()
        .cloned()
        .chain(replies.iter().cloned())
        .collect();
    if encode_payload(&combined, limits()).is_err() {
        return error(
            StatusCode::TOO_MANY_REQUESTS,
            "session queue byte limit reached",
        );
    }
    if close {
        session.notify.notify_waiters();
        sessions.remove(&sid);
    } else {
        session.connected = connected;
        if ping {
            session.last_ping = Instant::now();
        }
        session.queue.extend(replies);
        session.notify.notify_waiters();
    }
    text("ok".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    fn state() -> Server {
        Server {
            sessions: Arc::default(),
            ttl: Duration::from_secs(45),
            poll_wait: Duration::from_millis(10),
            capacity: 2,
        }
    }
    async fn call(router: &Router, method: &str, uri: &str, data: &str) -> (StatusCode, String) {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .body(Body::from(data.to_owned()))
                    .unwrap(),
            )
            .await
            .unwrap();
        (
            response.status(),
            String::from_utf8(
                to_bytes(response.into_body(), MAX_BODY)
                    .await
                    .unwrap()
                    .to_vec(),
            )
            .unwrap(),
        )
    }
    async fn open(router: &Router) -> String {
        let (status, body) = call(router, "GET", "/socket.io/?EIO=3&transport=polling", "").await;
        assert_eq!(status, StatusCode::OK);
        let packets = decode_payload(&body, limits()).unwrap();
        let data: serde_json::Value = serde_json::from_str(&packets[0].data).unwrap();
        assert_eq!(data["upgrades"], json!([]));
        data["sid"].as_str().unwrap().to_owned()
    }
    fn uri(sid: &str) -> String {
        format!("/socket.io/?EIO=3&transport=polling&sid={sid}")
    }
    async fn send(router: &Router, sid: &str, frames: &[EnginePacket]) -> StatusCode {
        call(
            router,
            "POST",
            &uri(sid),
            &encode_payload(frames, limits()).unwrap(),
        )
        .await
        .0
    }
    #[tokio::test]
    async fn lifecycle_isolation_close_and_heartbeat() {
        let app = build(state());
        let a = open(&app).await;
        let b = open(&app).await;
        assert_ne!(a, b);
        let connect = packet(EngineKind::Message, "0/compat,");
        assert_eq!(send(&app, &a, &[connect]).await, StatusCode::OK);
        assert_eq!(
            send(
                &app,
                &a,
                &[
                    packet(EngineKind::Message, "2/compat,[\"echo\",\"忍😀\"]"),
                    packet(EngineKind::Ping, "test")
                ]
            )
            .await,
            StatusCode::OK
        );
        let body = call(&app, "GET", &uri(&a), "").await.1;
        let replies = decode_payload(&body, limits()).unwrap();
        assert_eq!(replies.len(), 3);
        assert_eq!(replies[2], packet(EngineKind::Pong, "test"));
        let body = call(&app, "GET", &uri(&b), "").await.1;
        assert_eq!(
            decode_payload(&body, limits()).unwrap(),
            vec![packet(EngineKind::Noop, "")]
        );
        assert_eq!(
            send(&app, &a, &[packet(EngineKind::Close, "")]).await,
            StatusCode::OK
        );
        assert_eq!(
            call(&app, "GET", &uri(&a), "").await.0,
            StatusCode::BAD_REQUEST
        );
        assert_ne!(open(&app).await, a);
    }
    #[tokio::test]
    async fn malformed_requests_do_not_partially_connect() {
        let server = state();
        let app = build(server.clone());
        let sid = open(&app).await;
        for path in [
            "/socket.io/?EIO=4&transport=polling",
            "/socket.io/?EIO=3&transport=websocket",
            "/socket.io/?EIO=3&transport=polling&sid=bad",
        ] {
            assert_eq!(call(&app, "GET", path, "").await.0, StatusCode::BAD_REQUEST);
        }
        assert_eq!(
            call(&app, "POST", &uri(&sid), "999:4").await.0,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            send(
                &app,
                &sid,
                &[
                    packet(EngineKind::Message, "0/compat,"),
                    packet(EngineKind::Message, "0/other,")
                ]
            )
            .await,
            StatusCode::BAD_REQUEST
        );
        assert!(!server.sessions.lock().unwrap()[&sid].connected);
        assert_eq!(
            send(
                &app,
                &sid,
                &[packet(EngineKind::Message, "2/compat,[\"echo\"]")]
            )
            .await,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            call(&app, "POST", &uri(&sid), &"a".repeat(MAX_BODY + 1))
                .await
                .0,
            StatusCode::PAYLOAD_TOO_LARGE
        );
    }
    #[tokio::test]
    async fn capacity_expiry_and_queue_limits() {
        let server = state();
        let app = build(server.clone());
        let sid = open(&app).await;
        let _ = open(&app).await;
        assert_eq!(
            call(&app, "GET", "/socket.io/?EIO=3&transport=polling", "")
                .await
                .0,
            StatusCode::SERVICE_UNAVAILABLE
        );
        for _ in 0..MAX_QUEUE {
            assert_eq!(
                send(&app, &sid, &[packet(EngineKind::Ping, "")]).await,
                StatusCode::OK
            );
        }
        assert_eq!(
            send(&app, &sid, &[packet(EngineKind::Ping, "")]).await,
            StatusCode::TOO_MANY_REQUESTS
        );
        server
            .sessions
            .lock()
            .unwrap()
            .get_mut(&sid)
            .unwrap()
            .last_ping = Instant::now() - Duration::from_secs(46);
        assert_eq!(
            call(&app, "GET", &uri(&sid), "").await.0,
            StatusCode::BAD_REQUEST
        );
        let _ = open(&app).await;
    }
    #[tokio::test]
    async fn cancelled_poll_releases_exclusive_poll_slot() {
        let mut server = state();
        server.poll_wait = Duration::from_secs(20);
        let app = build(server.clone());
        let sid = open(&app).await;
        let pending_app = app.clone();
        let pending_sid = sid.clone();
        let task =
            tokio::spawn(async move { call(&pending_app, "GET", &uri(&pending_sid), "").await });
        for _ in 0..100 {
            if server.sessions.lock().unwrap()[&sid].polling {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert!(server.sessions.lock().unwrap()[&sid].polling);
        assert_eq!(
            call(&app, "GET", &uri(&sid), "").await.0,
            StatusCode::BAD_REQUEST
        );
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(!server.sessions.lock().unwrap()[&sid].polling);
    }
}
