//! HTTP REST routes for `/up`, `/enemy/{filename}`, `/skills/{filename}`, and `/clan/*`.
//!
//! Source:
//! - `routes/web.php`
//! - `app/Http/Middleware/ClanAuthMiddleware.php`
//! - `app/Http/Controllers/ClanApiController.php`
use crate::{GameState, rewards};
use authentication as auth;
use axum::{
    Json as AxumJson, Router,
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};
use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn router() -> Router<GameState> {
    Router::new()
        .route("/up", get(|| async { "OK" }))
        .route("/enemy/{filename}", get(serve_enemy_swf))
        .route("/skills/{filename}", get(serve_skill_swf))
        .route("/clan/season", post(clan_dispatch_root))
        .route("/clan/TapTap", post(clan_dispatch_root))
        .route("/clan/battle-histories", post(clan_dispatch_root))
        .route("/clan/opponents", post(clan_dispatch_root))
        .route("/clan/swap-master", post(clan_dispatch_root))
        .route("/clan/create", post(clan_dispatch_root))
        .route("/clan/rename", post(clan_dispatch_root))
        .route("/clan/season-histories", post(clan_dispatch_root))
        .route("/clan/{*rest}", post(clan_dispatch_rest))
}

async fn serve_enemy_swf(Path(filename): Path<String>) -> Response {
    if !filename.ends_with(".swf") || filename.contains("..") || filename.contains('/') {
        return StatusCode::NOT_FOUND.into_response();
    }
    let root = std::env::var("CLIENT_ASSET_ROOT").unwrap_or_else(|_| "public".into());
    let path = std::path::Path::new(&root).join("enemy").join(&filename);
    if let Ok(bytes) = tokio::fs::read(&path).await {
        return (
            [(header::CONTENT_TYPE, "application/x-shockwave-flash")],
            bytes,
        )
            .into_response();
    }
    StatusCode::NOT_FOUND.into_response()
}

async fn serve_skill_swf(State(state): State<GameState>, Path(filename): Path<String>) -> Response {
    if !filename.ends_with(".swf") || filename.contains("..") || filename.contains('/') {
        return StatusCode::NOT_FOUND.into_response();
    }
    let root = std::env::var("CLIENT_ASSET_ROOT").unwrap_or_else(|_| "public".into());
    let skills_dir = std::path::Path::new(&root).join("skills");
    let direct = skills_dir.join(&filename);
    if let Ok(bytes) = tokio::fs::read(&direct).await {
        return (
            [(header::CONTENT_TYPE, "application/x-shockwave-flash")],
            bytes,
        )
            .into_response();
    }
    let skill_id = filename.trim_end_matches(".swf");
    if let Some(pool) = &state.database
        && let Ok(Some(swf_name)) =
            sqlx::query_scalar::<_, Option<String>>("SELECT swf FROM skills WHERE skill_id=?")
                .bind(skill_id)
                .fetch_optional(pool)
                .await
        && let Some(swf) = swf_name.filter(|s| !s.is_empty())
    {
        let mapped = skills_dir.join(format!("{swf}.swf"));
        if let Ok(bytes) = tokio::fs::read(&mapped).await {
            return (
                [(header::CONTENT_TYPE, "application/x-shockwave-flash")],
                bytes,
            )
                .into_response();
        }
    }
    let fallback = skills_dir.join("skill_01.swf");
    if let Ok(bytes) = tokio::fs::read(&fallback).await {
        return (
            [(header::CONTENT_TYPE, "application/x-shockwave-flash")],
            bytes,
        )
            .into_response();
    }
    StatusCode::NOT_FOUND.into_response()
}

pub(crate) fn parse_body_params(
    headers: &HeaderMap,
    query: &HashMap<String, String>,
    bytes: &Bytes,
) -> HashMap<String, String> {
    let mut map = query.clone();
    if let Some(auth_hdr) = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        && let Some(bearer) = auth_hdr.strip_prefix("Bearer ")
    {
        map.entry("token".into())
            .or_insert_with(|| bearer.trim().into());
    }
    if bytes.is_empty() {
        return map;
    }
    if let Ok(Json::Object(obj)) = serde_json::from_slice::<Json>(bytes) {
        for (k, v) in obj {
            let val = match v {
                Json::String(s) => s,
                Json::Number(n) => n.to_string(),
                Json::Bool(b) => b.to_string(),
                other => other.to_string(),
            };
            map.insert(k, val);
        }
        return map;
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        for pair in text.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                map.insert(url_decode(k), url_decode(v));
            }
        }
    }
    map
}

fn url_decode(s: &str) -> String {
    let mut out = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let Ok(hex_str) = std::str::from_utf8(&bytes[i + 1..i + 3])
                    && let Ok(val) = u8::from_str_radix(hex_str, 16)
                {
                    out.push(val);
                    i += 3;
                    continue;
                }
                out.push(bytes[i]);
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub(crate) fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

async fn authenticate_token(
    pool: &MySqlPool,
    table: &str,
    token: &str,
) -> std::result::Result<Option<(i64, i64)>, sqlx::Error> {
    if token.trim().is_empty() {
        return Ok(None);
    }
    let q = format!(
        "SELECT user_id, character_id FROM {table} WHERE token=? AND (expires_at IS NULL OR expires_at > NOW()) LIMIT 1"
    );
    let row = sqlx::query(&q)
        .bind(token.trim())
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| {
        (
            r.get::<u64, _>("user_id") as i64,
            r.get::<u64, _>("character_id") as i64,
        )
    }))
}

async fn clan_dispatch_root(
    State(state): State<GameState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
    uri: axum::http::Uri,
    bytes: Bytes,
) -> Response {
    let rest = uri.path().strip_prefix("/clan/").unwrap_or("");
    handle_clan(state, headers, query, rest.to_string(), bytes).await
}

async fn clan_dispatch_rest(
    State(state): State<GameState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
    Path(rest): Path<String>,
    bytes: Bytes,
) -> Response {
    handle_clan(state, headers, query, rest, bytes).await
}

async fn handle_clan(
    state: GameState,
    headers: HeaderMap,
    query: HashMap<String, String>,
    rest: String,
    bytes: Bytes,
) -> Response {
    let Some(pool) = &state.database else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            AxumJson(json!({"errorMessage": "Database not configured"})),
        )
            .into_response();
    };
    let params = parse_body_params(&headers, &query, &bytes);
    let clean_rest = rest.trim_matches('/');
    if clean_rest == "season" {
        return match ensure_clan_season(pool).await {
            Ok((sid, snum)) => AxumJson(json!({
                "id": sid,
                "number": snum,
                "active": true,
                "timestamp": 86400,
                "season": {
                    "id": sid,
                    "number": snum,
                    "season": snum,
                    "active": true,
                    "timestamp": 86400,
                }
            }))
            .into_response(),
            Err(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                AxumJson(json!({"errorMessage": "Database operation failed"})),
            )
                .into_response(),
        };
    }
    if clean_rest == "auth/login" {
        return handle_clan_login(pool, &params).await;
    }
    let token = params.get("token").map(String::as_str).unwrap_or("");
    let auth_res = match authenticate_token(pool, "clan_auth_tokens", token).await {
        Ok(Some(ids)) => ids,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                AxumJson(json!({"errorMessage": "Invalid or expired token", "code": 401})),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                AxumJson(json!({"errorMessage": "Database error"})),
            )
                .into_response();
        }
    };
    let (user_id, char_id) = auth_res;
    match dispatch_clan_route(pool, user_id, char_id, clean_rest, &params).await {
        Ok(val) => AxumJson(val).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            AxumJson(json!({"errorMessage": "Database operation failed"})),
        )
            .into_response(),
    }
}

pub(crate) async fn verify_char_session_and_issue_token(
    pool: &MySqlPool,
    table: &str,
    params: &HashMap<String, String>,
) -> Response {
    let char_id = params
        .get("char_id")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);
    let session_key = params.get("session_key").map(|s| s.trim()).unwrap_or("");
    if char_id <= 0 || session_key.is_empty() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            AxumJson(json!({"status": 0, "errorMessage": "Missing credentials"})),
        )
            .into_response();
    }
    let Some(c_row) = sqlx::query("SELECT user_id FROM characters WHERE id=?")
        .bind(char_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
    else {
        return (
            StatusCode::UNAUTHORIZED,
            AxumJson(json!({"status": 0, "errorMessage": "Invalid character"})),
        )
            .into_response();
    };
    let user_id: u64 = c_row.get("user_id");
    let Some(u_row) = sqlx::query("SELECT session_key FROM users WHERE id=?")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
    else {
        return (
            StatusCode::UNAUTHORIZED,
            AxumJson(json!({"status": 0, "errorMessage": "Invalid session key"})),
        )
            .into_response();
    };
    let stored = u_row
        .get::<Option<String>, _>("session_key")
        .unwrap_or_default();
    if stored.is_empty() || !auth::matches_session(user_id, &stored, session_key) {
        return (
            StatusCode::UNAUTHORIZED,
            AxumJson(json!({"status": 0, "errorMessage": "Invalid session key"})),
        )
            .into_response();
    }
    let _ = sqlx::query(&format!("DELETE FROM {table} WHERE character_id=?"))
        .bind(char_id)
        .execute(pool)
        .await;
    let token = format!(
        "{}{}",
        auth::random_hex().unwrap_or_default(),
        auth::random_hex().unwrap_or_default()
    );
    let _ = sqlx::query(&format!(
        "INSERT INTO {table} (user_id, character_id, token, expires_at, created_at, updated_at) \
         VALUES (?, ?, ?, DATE_ADD(NOW(), INTERVAL 24 HOUR), NOW(), NOW())"
    ))
    .bind(user_id)
    .bind(char_id)
    .bind(&token)
    .execute(pool)
    .await;
    AxumJson(json!({"status": 1, "access_token": token})).into_response()
}

async fn handle_clan_login(pool: &MySqlPool, params: &HashMap<String, String>) -> Response {
    verify_char_session_and_issue_token(pool, "clan_auth_tokens", params).await
}

async fn ensure_clan_season(pool: &MySqlPool) -> std::result::Result<(u64, u32), sqlx::Error> {
    if let Some(row) = sqlx::query("SELECT id, number FROM clan_seasons WHERE active=1 LIMIT 1")
        .fetch_optional(pool)
        .await?
    {
        return Ok((row.get("id"), row.get("number")));
    }
    let res = sqlx::query(
        "INSERT INTO clan_seasons (number, active, started_at, created_at, updated_at) VALUES (1, 1, NOW(), NOW(), NOW())",
    )
    .execute(pool)
    .await?;
    Ok((res.last_insert_id(), 1))
}

async fn format_clan_full(
    pool: &MySqlPool,
    clan_id: u64,
    char_id: i64,
) -> std::result::Result<Option<Json>, sqlx::Error> {
    let Some(c_row) = sqlx::query("SELECT * FROM clans WHERE id=?")
        .bind(clan_id)
        .fetch_optional(pool)
        .await?
    else {
        return Ok(None);
    };
    let master_id = c_row.get::<u64, _>("master_id") as i64;
    let master_name: Option<String> = sqlx::query_scalar("SELECT name FROM characters WHERE id=?")
        .bind(master_id)
        .fetch_optional(pool)
        .await?;
    let elder_row = sqlx::query(
        "SELECT cm.character_id, c.name FROM clan_members cm JOIN characters c ON c.id = cm.character_id WHERE cm.clan_id=? AND cm.role='elder' LIMIT 1",
    )
    .bind(clan_id)
    .fetch_optional(pool)
    .await?;
    let (elder_id, elder_name) = match elder_row {
        Some(r) => (
            Some(r.get::<u64, _>("character_id")),
            Some(r.get::<String, _>("name")),
        ),
        None => (None, None),
    };
    let members_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM clan_members WHERE clan_id=?")
            .bind(clan_id)
            .fetch_one(pool)
            .await?;
    let m_row = sqlx::query(
        "SELECT role, stamina, max_stamina FROM clan_members WHERE clan_id=? AND character_id=?",
    )
    .bind(clan_id)
    .bind(char_id)
    .fetch_optional(pool)
    .await?;
    let (role, stamina, max_stamina) = match m_row {
        Some(r) => (
            r.get::<String, _>("role"),
            r.get::<u32, _>("stamina"),
            r.get::<u32, _>("max_stamina"),
        ),
        None => ("member".into(), 0, 5),
    };
    let b_json = c_row
        .try_get::<Option<Json>, _>("buildings")
        .ok()
        .flatten()
        .unwrap_or_else(|| json!({}));
    let prestige = c_row.get::<u32, _>("prestige");

    Ok(Some(json!({
        "id": clan_id,
        "name": c_row.get::<String, _>("name"),
        "prestige": prestige,
        "reputation": prestige,
        "master_id": master_id,
        "master_name": master_name,
        "elder_id": elder_id,
        "elder_name": elder_name,
        "max_members": c_row.get::<u32, _>("max_members"),
        "members": members_count,
        "golds": c_row.get::<u64, _>("gold"),
        "tokens": c_row.get::<u64, _>("tokens"),
        "announcement": c_row.get::<Option<String>, _>("announcement_published").unwrap_or_default(),
        "announcement_draft": c_row.get::<Option<String>, _>("announcement_draft").unwrap_or_default(),
        "role": role,
        "stamina": stamina,
        "max_stamina": max_stamina,
        "season_id": c_row.get::<Option<u64>, _>("season_id"),
        "ramen": b_json.get("ramen").and_then(Json::as_i64).unwrap_or(0),
        "hot_spring": b_json.get("hot_spring").and_then(Json::as_i64).unwrap_or(0),
        "temple": b_json.get("temple").and_then(Json::as_i64).unwrap_or(0),
        "training_hall": b_json.get("training_hall").and_then(Json::as_i64).unwrap_or(0),
    })))
}

async fn format_clan_members(
    pool: &MySqlPool,
    clan_id: u64,
) -> std::result::Result<Vec<Json>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT cm.character_id, cm.role, cm.stamina, cm.max_stamina, cm.donated_golds, cm.donated_tokens, \
                UNIX_TIMESTAMP(cm.prestige_boost_expires_at) AS boost_ts, c.name, c.level \
         FROM clan_members cm LEFT JOIN characters c ON c.id = cm.character_id WHERE cm.clan_id=?",
    )
    .bind(clan_id)
    .fetch_all(pool)
    .await?;
    let now = now_ts();
    Ok(rows
        .into_iter()
        .map(|r| {
            let boost_ts = r
                .try_get::<Option<i64>, _>("boost_ts")
                .ok()
                .flatten()
                .unwrap_or(0);
            json!({
                "char_id": r.get::<u64, _>("character_id"),
                "name": r.get::<Option<String>, _>("name").unwrap_or_else(|| "Unknown".into()),
                "level": r.try_get::<Option<i32>, _>("level").ok().flatten().unwrap_or(1),
                "stamina": r.get::<u32, _>("stamina"),
                "max_stamina": r.get::<u32, _>("max_stamina"),
                "reputation": 0,
                "gold_donated": r.get::<u64, _>("donated_golds"),
                "token_donated": r.get::<u64, _>("donated_tokens"),
                "role": r.get::<String, _>("role"),
                "prestige": 0,
                "prestige_boost": (boost_ts - now).max(0),
            })
        })
        .collect())
}

async fn format_clan_summaries(
    pool: &MySqlPool,
    where_clause: &str,
    bind_val: Option<String>,
    limit: i64,
) -> std::result::Result<Vec<Json>, sqlx::Error> {
    let sql = format!(
        "SELECT c.id, c.name, c.prestige, c.max_members, \
         (SELECT COUNT(*) FROM clan_members cm WHERE cm.clan_id = c.id) AS m_cnt \
         FROM clans c {where_clause} ORDER BY c.prestige DESC LIMIT {limit}"
    );
    let rows = if let Some(b) = bind_val {
        sqlx::query(&sql).bind(b).fetch_all(pool).await?
    } else {
        sqlx::query(&sql).fetch_all(pool).await?
    };
    Ok(rows
        .into_iter()
        .map(|r| {
            json!({
                "id": r.get::<u64, _>("id"),
                "name": r.get::<String, _>("name"),
                "reputation": r.get::<u32, _>("prestige"),
                "max_members": r.get::<u32, _>("max_members"),
                "members": r.get::<i64, _>("m_cnt"),
            })
        })
        .collect())
}

async fn dispatch_clan_route(
    pool: &MySqlPool,
    user_id: i64,
    char_id: i64,
    path: &str,
    params: &HashMap<String, String>,
) -> std::result::Result<Json, sqlx::Error> {
    let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    match segs.as_slice() {
        ["season"] => {
            let (sid, snum) = ensure_clan_season(pool).await?;
            Ok(json!({
                "season": {
                    "id": sid,
                    "season": snum,
                    "active": true,
                    "timestamp": 86400,
                }
            }))
        }
        ["TapTap"] => Ok(json!({ "TapTap": 0 })),
        ["player", "data"] => {
            let member = sqlx::query(
                "SELECT clan_id, role, stamina, max_stamina, donated_golds, donated_tokens, \
                 UNIX_TIMESTAMP(prestige_boost_expires_at) AS boost_ts \
                 FROM clan_members WHERE character_id=?",
            )
            .bind(char_id)
            .fetch_optional(pool)
            .await?;
            let Some(m) = member else {
                return Ok(json!({ "clan_data": null }));
            };
            let clan_id: u64 = m.get("clan_id");
            let clan_data = format_clan_full(pool, clan_id, char_id).await?;
            let c_row = sqlx::query("SELECT name, level FROM characters WHERE id=?")
                .bind(char_id)
                .fetch_optional(pool)
                .await?;
            let (c_name, c_lvl) = match c_row {
                Some(r) => (r.get::<String, _>("name"), r.get::<i32, _>("level")),
                None => ("Unknown".into(), 1),
            };
            let boost_ts = m
                .try_get::<Option<i64>, _>("boost_ts")
                .ok()
                .flatten()
                .unwrap_or(0);
            let onigiri_qty: i32 = sqlx::query_scalar(
                "SELECT quantity FROM character_items WHERE character_id=? AND item_id='material_69'",
            )
            .bind(char_id)
            .fetch_optional(pool)
            .await?
            .unwrap_or(0);

            Ok(json!({
                "clan_data": clan_data,
                "char_data": {
                    "char_id": char_id,
                    "name": c_name,
                    "level": c_lvl,
                    "stamina": m.get::<u32, _>("stamina"),
                    "max_stamina": m.get::<u32, _>("max_stamina"),
                    "reputation": 0,
                    "gold_donated": m.get::<u64, _>("donated_golds"),
                    "token_donated": m.get::<u64, _>("donated_tokens"),
                    "role": m.get::<String, _>("role"),
                    "prestige": 0,
                    "prestige_boost": (boost_ts - now_ts()).max(0),
                    "onigiri_data": onigiri_qty,
                }
            }))
        }
        ["player", "clan"] => {
            let member = sqlx::query("SELECT clan_id FROM clan_members WHERE character_id=?")
                .bind(char_id)
                .fetch_optional(pool)
                .await?;
            let Some(m) = member else {
                let clans = format_clan_summaries(pool, "", None, 50).await?;
                return Ok(json!({ "clan": null, "clans": clans }));
            };
            let clan_id: u64 = m.get("clan_id");
            let clan = format_clan_full(pool, clan_id, char_id).await?;
            let members = format_clan_members(pool, clan_id).await?;
            let (sid, _) = ensure_clan_season(pool).await?;
            Ok(json!({
                "clan": clan,
                "members": members,
                "date": sid,
            }))
        }
        ["battle-histories"] | ["history"] => {
            let member = sqlx::query("SELECT clan_id FROM clan_members WHERE character_id=?")
                .bind(char_id)
                .fetch_optional(pool)
                .await?;
            let Some(m) = member else {
                return Ok(json!({ "histories": [] }));
            };
            let clan_id: u64 = m.get("clan_id");
            let rows = sqlx::query(
                "SELECT cb.*, a.name AS attacker_name, d.name AS defender_name \
                 FROM clan_battles cb \
                 LEFT JOIN clans a ON a.id = cb.attacker_clan_id \
                 LEFT JOIN clans d ON d.id = cb.defender_clan_id \
                 WHERE cb.attacker_clan_id=? OR cb.defender_clan_id=? \
                 ORDER BY cb.created_at DESC, cb.id DESC LIMIT 20",
            )
            .bind(clan_id)
            .bind(clan_id)
            .fetch_all(pool)
            .await?;
            let histories: Vec<Json> = rows
                .into_iter()
                .map(|r| {
                    let att_id: u64 = r.get("attacker_clan_id");
                    let def_id: u64 = r.get("defender_clan_id");
                    let is_att = att_id == clan_id;
                    let att_won: bool = r.get("attacker_won");
                    let won = if is_att { att_won } else { !att_won };
                    let opp_id = if is_att { def_id } else { att_id };
                    let opp_name = if is_att {
                        r.get::<Option<String>, _>("defender_name")
                    } else {
                        r.get::<Option<String>, _>("attacker_name")
                    }
                    .unwrap_or_else(|| "Unknown".into());
                    let rep_str = if is_att {
                        if won { "+10" } else { "0" }
                    } else if won {
                        "0"
                    } else {
                        "-10"
                    };
                    json!({
                        "detail": format!("Clan {} ({})", opp_name, opp_id),
                        "True": if won { 1 } else { 0 },
                        "reputation": rep_str,
                        "date": "just now",
                    })
                })
                .collect();
            Ok(json!({ "histories": histories }))
        }
        ["opponents"] => {
            let clan_id: u64 =
                sqlx::query_scalar("SELECT clan_id FROM clan_members WHERE character_id=?")
                    .bind(char_id)
                    .fetch_optional(pool)
                    .await?
                    .unwrap_or(0);
            let clans =
                format_clan_summaries(pool, "WHERE c.id != ?", Some(clan_id.to_string()), 10)
                    .await?;
            Ok(json!({ "clans": clans }))
        }
        ["opponents", id_str] => {
            let clans =
                format_clan_summaries(pool, "WHERE c.id = ?", Some((*id_str).into()), 1).await?;
            Ok(json!({ "clans": clans }))
        }
        ["request", "Filtered"] => {
            let name = params.get("name").cloned().unwrap_or_default();
            let id = params.get("id").cloned().unwrap_or_default();
            let clans = if !id.is_empty() {
                format_clan_summaries(pool, "WHERE c.id = ?", Some(id), 20).await?
            } else if !name.is_empty() {
                format_clan_summaries(pool, "WHERE c.name LIKE ?", Some(format!("%{name}%")), 20)
                    .await?
            } else {
                format_clan_summaries(pool, "", None, 20).await?
            };
            Ok(json!({ "clans": clans }))
        }
        ["request", "all"] => {
            let clans = format_clan_summaries(pool, "", None, 50).await?;
            Ok(json!({ "clans": clans }))
        }
        ["request", "send", clan_id_str] => {
            let target_clan_id = clan_id_str.parse::<u64>().unwrap_or(0);
            let in_clan: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM clan_members WHERE character_id=?")
                    .bind(char_id)
                    .fetch_one(pool)
                    .await?;
            if in_clan > 0 {
                return Ok(json!({ "errorMessage": "Already in a clan" }));
            }
            let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM clans WHERE id=?")
                .bind(target_clan_id)
                .fetch_one(pool)
                .await?;
            if exists == 0 {
                return Ok(json!({ "errorMessage": "Clan not found" }));
            }
            let pending: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM clan_requests WHERE clan_id=? AND character_id=? AND status='pending'",
            )
            .bind(target_clan_id)
            .bind(char_id)
            .fetch_one(pool)
            .await?;
            if pending == 0 {
                sqlx::query(
                    "INSERT INTO clan_requests (clan_id, character_id, status, created_at, updated_at) VALUES (?, ?, 'pending', NOW(), NOW())",
                )
                .bind(target_clan_id)
                .bind(char_id)
                .execute(pool)
                .await?;
            }
            Ok(json!({ "result": "Request to join clan sent!" }))
        }
        ["request", "available"] => {
            let member = sqlx::query("SELECT clan_id, role FROM clan_members WHERE character_id=?")
                .bind(char_id)
                .fetch_optional(pool)
                .await?;
            let Some(m) = member else {
                return Ok(json!({ "requests": [] }));
            };
            let role: String = m.get("role");
            if role != "master" && role != "elder" {
                return Ok(json!({ "requests": [] }));
            }
            let clan_id: u64 = m.get("clan_id");
            let rows = sqlx::query(
                "SELECT cr.id, cr.character_id, c.name, c.level \
                 FROM clan_requests cr LEFT JOIN characters c ON c.id = cr.character_id \
                 WHERE cr.clan_id=? AND cr.status='pending'",
            )
            .bind(clan_id)
            .fetch_all(pool)
            .await?;
            let requests: Vec<Json> = rows
                .into_iter()
                .map(|r| {
                    json!({
                        "id": r.get::<u64, _>("id"),
                        "char_id": r.get::<u64, _>("character_id"),
                        "name": r.get::<Option<String>, _>("name").unwrap_or_else(|| "Unknown".into()),
                        "level": r.try_get::<Option<i32>, _>("level").ok().flatten().unwrap_or(1),
                    })
                })
                .collect();
            Ok(json!({ "requests": requests }))
        }
        ["request", req_id_str, "reject"] => {
            let req_id = req_id_str.parse::<u64>().unwrap_or(0);
            let member = sqlx::query("SELECT clan_id, role FROM clan_members WHERE character_id=?")
                .bind(char_id)
                .fetch_optional(pool)
                .await?;
            let Some(m) = member else {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            };
            let role: String = m.get("role");
            if role != "master" && role != "elder" {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            }
            sqlx::query("UPDATE clan_requests SET status='rejected', updated_at=NOW() WHERE id=? AND clan_id=?")
                .bind(req_id)
                .bind(m.get::<u64, _>("clan_id"))
                .execute(pool)
                .await?;
            Ok(json!({ "success": true }))
        }
        ["request", "reject-all"] => {
            let member = sqlx::query("SELECT clan_id, role FROM clan_members WHERE character_id=?")
                .bind(char_id)
                .fetch_optional(pool)
                .await?;
            let Some(m) = member else {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            };
            let role: String = m.get("role");
            if role != "master" && role != "elder" {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            }
            sqlx::query("UPDATE clan_requests SET status='rejected', updated_at=NOW() WHERE clan_id=? AND status='pending'")
                .bind(m.get::<u64, _>("clan_id"))
                .execute(pool)
                .await?;
            Ok(json!({ "success": true }))
        }
        ["request", req_id_str, "accept"] => {
            let req_id = req_id_str.parse::<u64>().unwrap_or(0);
            let member = sqlx::query("SELECT clan_id, role FROM clan_members WHERE character_id=?")
                .bind(char_id)
                .fetch_optional(pool)
                .await?;
            let Some(m) = member else {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            };
            let role: String = m.get("role");
            if role != "master" && role != "elder" {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            }
            let clan_id: u64 = m.get("clan_id");
            let Some(req_row) = sqlx::query(
                "SELECT character_id FROM clan_requests WHERE id=? AND clan_id=? AND status='pending'",
            )
            .bind(req_id)
            .bind(clan_id)
            .fetch_optional(pool)
            .await?
            else {
                return Ok(json!({ "errorMessage": "Request not found" }));
            };
            let target_char_id: u64 = req_row.get("character_id");
            let max_members: u32 = sqlx::query_scalar("SELECT max_members FROM clans WHERE id=?")
                .bind(clan_id)
                .fetch_one(pool)
                .await?;
            let cur_cnt: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM clan_members WHERE clan_id=?")
                    .bind(clan_id)
                    .fetch_one(pool)
                    .await?;
            if cur_cnt >= i64::from(max_members) {
                return Ok(json!({ "errorMessage": "Clan is full" }));
            }
            let mut tx = pool.begin().await?;
            sqlx::query("DELETE FROM clan_members WHERE character_id=?")
                .bind(target_char_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "INSERT INTO clan_members (clan_id, character_id, role, stamina, max_stamina, created_at, updated_at) \
                 VALUES (?, ?, 'member', 5, 5, NOW(), NOW())",
            )
            .bind(clan_id)
            .bind(target_char_id)
            .execute(&mut *tx)
            .await?;
            sqlx::query("UPDATE clan_requests SET status='accepted', updated_at=NOW() WHERE id=?")
                .bind(req_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE clan_requests SET status='rejected', updated_at=NOW() WHERE character_id=? AND id!=? AND status='pending'")
                .bind(target_char_id)
                .bind(req_id)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            Ok(json!({ "success": true }))
        }
        ["player", "quit"] => {
            let member = sqlx::query("SELECT role FROM clan_members WHERE character_id=?")
                .bind(char_id)
                .fetch_optional(pool)
                .await?;
            let Some(m) = member else {
                return Ok(json!({ "errorMessage": "Not in a clan" }));
            };
            if m.get::<String, _>("role") == "master" {
                return Ok(
                    json!({ "errorMessage": "Master cannot quit. Transfer leadership first." }),
                );
            }
            sqlx::query("DELETE FROM clan_members WHERE character_id=?")
                .bind(char_id)
                .execute(pool)
                .await?;
            Ok(json!({ "success": true }))
        }
        ["member", target_str, "kick"] => {
            let target_id = target_str.parse::<i64>().unwrap_or(0);
            let actor = sqlx::query("SELECT clan_id, role FROM clan_members WHERE character_id=?")
                .bind(char_id)
                .fetch_optional(pool)
                .await?;
            let Some(actor) = actor else {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            };
            let a_role: String = actor.get("role");
            if a_role != "master" && a_role != "elder" {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            }
            let clan_id: u64 = actor.get("clan_id");
            let Some(target) =
                sqlx::query("SELECT role FROM clan_members WHERE clan_id=? AND character_id=?")
                    .bind(clan_id)
                    .bind(target_id)
                    .fetch_optional(pool)
                    .await?
            else {
                return Ok(json!({ "errorMessage": "Member not found" }));
            };
            let t_role: String = target.get("role");
            if t_role == "master" || (a_role == "elder" && t_role == "elder") {
                return Ok(json!({ "errorMessage": "Cannot kick this member" }));
            }
            sqlx::query("DELETE FROM clan_members WHERE clan_id=? AND character_id=?")
                .bind(clan_id)
                .bind(target_id)
                .execute(pool)
                .await?;
            Ok(json!({ "success": true }))
        }
        ["swap-master"] => {
            let new_master_id = params
                .get("character_id")
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            let actor = sqlx::query("SELECT clan_id, role FROM clan_members WHERE character_id=?")
                .bind(char_id)
                .fetch_optional(pool)
                .await?;
            let Some(actor) = actor else {
                return Ok(json!({ "errorMessage": "Only master can transfer leadership" }));
            };
            if actor.get::<String, _>("role") != "master" {
                return Ok(json!({ "errorMessage": "Only master can transfer leadership" }));
            }
            let clan_id: u64 = actor.get("clan_id");
            let target_exists: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM clan_members WHERE clan_id=? AND character_id=?",
            )
            .bind(clan_id)
            .bind(new_master_id)
            .fetch_one(pool)
            .await?;
            if target_exists == 0 {
                return Ok(json!({ "errorMessage": "Member not found" }));
            }
            let mut tx = pool.begin().await?;
            sqlx::query("UPDATE clan_members SET role='member', updated_at=NOW() WHERE clan_id=? AND character_id=?")
                .bind(clan_id)
                .bind(char_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE clan_members SET role='master', updated_at=NOW() WHERE clan_id=? AND character_id=?")
                .bind(clan_id)
                .bind(new_master_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE clans SET master_id=?, updated_at=NOW() WHERE id=?")
                .bind(new_master_id)
                .bind(clan_id)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            Ok(json!({ "success": true }))
        }
        ["member", target_str, "promote-elder"] => {
            let target_id = target_str.parse::<i64>().unwrap_or(0);
            let actor = sqlx::query("SELECT clan_id, role FROM clan_members WHERE character_id=?")
                .bind(char_id)
                .fetch_optional(pool)
                .await?;
            let Some(actor) = actor else {
                return Ok(json!({ "errorMessage": "Only master can promote" }));
            };
            if actor.get::<String, _>("role") != "master" {
                return Ok(json!({ "errorMessage": "Only master can promote" }));
            }
            let clan_id: u64 = actor.get("clan_id");
            let Some(target) =
                sqlx::query("SELECT role FROM clan_members WHERE clan_id=? AND character_id=?")
                    .bind(clan_id)
                    .bind(target_id)
                    .fetch_optional(pool)
                    .await?
            else {
                return Ok(json!({ "errorMessage": "Member not found" }));
            };
            let new_role = if target.get::<String, _>("role") == "elder" {
                "member"
            } else {
                "elder"
            };
            sqlx::query("UPDATE clan_members SET role=?, updated_at=NOW() WHERE clan_id=? AND character_id=?")
                .bind(new_role)
                .bind(clan_id)
                .bind(target_id)
                .execute(pool)
                .await?;
            Ok(json!({ "success": true, "role": new_role }))
        }
        ["player", "donate", amount_str, "golds"] => {
            let amount = amount_str.parse::<i64>().unwrap_or(0);
            if amount <= 0 {
                return Ok(json!({ "errorMessage": "Invalid amount" }));
            }
            let mut tx = pool.begin().await?;
            let Some(m) =
                sqlx::query("SELECT clan_id FROM clan_members WHERE character_id=? FOR UPDATE")
                    .bind(char_id)
                    .fetch_optional(&mut *tx)
                    .await?
            else {
                return Ok(json!({ "errorMessage": "Not in a clan" }));
            };
            let clan_id: u64 = m.get("clan_id");
            let gold = i64::from(
                sqlx::query_scalar::<_, i32>("SELECT gold FROM characters WHERE id=? FOR UPDATE")
                    .bind(char_id)
                    .fetch_one(&mut *tx)
                    .await?,
            );
            if gold < amount {
                return Ok(json!({ "errorMessage": "Not enough gold" }));
            }
            let new_gold = gold - amount;
            sqlx::query("UPDATE characters SET gold=?, updated_at=NOW() WHERE id=?")
                .bind(new_gold)
                .bind(char_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE clan_members SET donated_golds = donated_golds + ?, updated_at=NOW() WHERE character_id=?")
                .bind(amount)
                .bind(char_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE clans SET gold = gold + ?, updated_at=NOW() WHERE id=?")
                .bind(amount)
                .bind(clan_id)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            Ok(json!({ "success": true, "gold": new_gold }))
        }
        ["player", "donate", amount_str, "tokens"] => {
            let amount = amount_str.parse::<i64>().unwrap_or(0);
            if amount <= 0 {
                return Ok(json!({ "errorMessage": "Invalid amount" }));
            }
            let mut tx = pool.begin().await?;
            let Some(m) =
                sqlx::query("SELECT clan_id FROM clan_members WHERE character_id=? FOR UPDATE")
                    .bind(char_id)
                    .fetch_optional(&mut *tx)
                    .await?
            else {
                return Ok(json!({ "errorMessage": "Not in a clan" }));
            };
            let clan_id: u64 = m.get("clan_id");
            let tokens = i64::from(
                sqlx::query_scalar::<_, i32>("SELECT tokens FROM users WHERE id=? FOR UPDATE")
                    .bind(user_id)
                    .fetch_one(&mut *tx)
                    .await?,
            );
            if tokens < amount {
                return Ok(json!({ "errorMessage": "Not enough tokens" }));
            }
            let new_tokens = tokens - amount;
            sqlx::query("UPDATE users SET tokens=?, updated_at=NOW() WHERE id=?")
                .bind(new_tokens)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE clan_members SET donated_tokens = donated_tokens + ?, updated_at=NOW() WHERE character_id=?")
                .bind(amount)
                .bind(char_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE clans SET tokens = tokens + ?, updated_at=NOW() WHERE id=?")
                .bind(amount)
                .bind(clan_id)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            Ok(json!({ "success": true, "tokens": new_tokens }))
        }
        ["upgrade-building", building_id] => {
            let b_key = building_id.strip_prefix("clan_").unwrap_or(building_id);
            if !matches!(b_key, "ramen" | "hot_spring" | "temple" | "training_hall") {
                return Ok(json!({ "errorMessage": "Invalid building" }));
            }
            let mut tx = pool.begin().await?;
            let Some(m) = sqlx::query(
                "SELECT clan_id, role FROM clan_members WHERE character_id=? FOR UPDATE",
            )
            .bind(char_id)
            .fetch_optional(&mut *tx)
            .await?
            else {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            };
            let role: String = m.get("role");
            if role != "master" && role != "elder" {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            }
            let clan_id: u64 = m.get("clan_id");
            let c_row =
                sqlx::query("SELECT gold, tokens, buildings FROM clans WHERE id=? FOR UPDATE")
                    .bind(clan_id)
                    .fetch_one(&mut *tx)
                    .await?;
            let c_gold = c_row.get::<u64, _>("gold") as i64;
            let c_tokens = c_row.get::<u64, _>("tokens") as i64;
            let mut buildings = c_row
                .try_get::<Option<Json>, _>("buildings")
                .ok()
                .flatten()
                .and_then(|v| v.as_object().cloned())
                .unwrap_or_default();
            let cur_lvl = buildings.get(b_key).and_then(Json::as_i64).unwrap_or(0);
            if cur_lvl >= 3 {
                return Ok(json!({ "errorMessage": "Building is already at max level" }));
            }
            if cur_lvl < 2 {
                let cost = (cur_lvl + 1) * 1000;
                if c_gold < cost {
                    return Ok(json!({ "errorMessage": "Not enough clan gold" }));
                }
                buildings.insert(b_key.into(), json!(cur_lvl + 1));
                sqlx::query("UPDATE clans SET gold=?, buildings=?, updated_at=NOW() WHERE id=?")
                    .bind(c_gold - cost)
                    .bind(serde_json::to_string(&buildings).unwrap_or_else(|_| "{}".into()))
                    .bind(clan_id)
                    .execute(&mut *tx)
                    .await?;
            } else {
                let cost = 4000;
                if c_tokens < cost {
                    return Ok(json!({ "errorMessage": "Not enough clan tokens" }));
                }
                buildings.insert(b_key.into(), json!(cur_lvl + 1));
                sqlx::query("UPDATE clans SET tokens=?, buildings=?, updated_at=NOW() WHERE id=?")
                    .bind(c_tokens - cost)
                    .bind(serde_json::to_string(&buildings).unwrap_or_else(|_| "{}".into()))
                    .bind(clan_id)
                    .execute(&mut *tx)
                    .await?;
            }
            tx.commit().await?;
            Ok(json!({ "success": true }))
        }
        ["announcement", "save"] => {
            let member = sqlx::query("SELECT clan_id, role FROM clan_members WHERE character_id=?")
                .bind(char_id)
                .fetch_optional(pool)
                .await?;
            let Some(m) = member else {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            };
            let role: String = m.get("role");
            if role != "master" && role != "elder" {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            }
            let ann = params.get("announcement").cloned().unwrap_or_default();
            sqlx::query("UPDATE clans SET announcement_draft=?, updated_at=NOW() WHERE id=?")
                .bind(ann)
                .bind(m.get::<u64, _>("clan_id"))
                .execute(pool)
                .await?;
            Ok(json!({ "success": true }))
        }
        ["announcement", "publish"] => {
            let member = sqlx::query("SELECT clan_id, role FROM clan_members WHERE character_id=?")
                .bind(char_id)
                .fetch_optional(pool)
                .await?;
            let Some(m) = member else {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            };
            let role: String = m.get("role");
            if role != "master" && role != "elder" {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            }
            sqlx::query(
                "UPDATE clans SET announcement_published = announcement_draft, updated_at=NOW() WHERE id=?",
            )
            .bind(m.get::<u64, _>("clan_id"))
            .execute(pool)
            .await?;
            Ok(json!("ok"))
        }
        ["member", "increase-max-members"] => {
            let mut tx = pool.begin().await?;
            let Some(m) = sqlx::query(
                "SELECT clan_id, role FROM clan_members WHERE character_id=? FOR UPDATE",
            )
            .bind(char_id)
            .fetch_optional(&mut *tx)
            .await?
            else {
                return Ok(json!({ "errorMessage": "Only master can increase member cap" }));
            };
            if m.get::<String, _>("role") != "master" {
                return Ok(json!({ "errorMessage": "Only master can increase member cap" }));
            }
            let clan_id: u64 = m.get("clan_id");
            let c_row = sqlx::query("SELECT max_members, tokens FROM clans WHERE id=? FOR UPDATE")
                .bind(clan_id)
                .fetch_one(&mut *tx)
                .await?;
            let cur_max = i64::from(c_row.get::<u32, _>("max_members"));
            let c_tokens = c_row.get::<u64, _>("tokens") as i64;
            if cur_max >= 45 {
                return Ok(json!({ "errorMessage": "Already at maximum member capacity" }));
            }
            let new_max = (cur_max + 10).min(45);
            let cost = new_max * 10;
            if c_tokens < cost {
                return Ok(json!({ "errorMessage": "Not enough clan tokens" }));
            }
            sqlx::query("UPDATE clans SET max_members=?, tokens=?, updated_at=NOW() WHERE id=?")
                .bind(new_max)
                .bind(c_tokens - cost)
                .bind(clan_id)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            Ok(json!({ "success": true, "max_members": new_max }))
        }
        ["player", "stamina", "upgrade-max"] => {
            let mut tx = pool.begin().await?;
            let Some(m) =
                sqlx::query("SELECT max_stamina FROM clan_members WHERE character_id=? FOR UPDATE")
                    .bind(char_id)
                    .fetch_optional(&mut *tx)
                    .await?
            else {
                return Ok(json!({ "errorMessage": "Not in a clan" }));
            };
            let cur_max = i64::from(m.get::<u32, _>("max_stamina"));
            let tokens = i64::from(
                sqlx::query_scalar::<_, i32>("SELECT tokens FROM users WHERE id=? FOR UPDATE")
                    .bind(user_id)
                    .fetch_one(&mut *tx)
                    .await?,
            );
            if tokens < 500 {
                return Ok(json!({ "errorMessage": "Not enough tokens" }));
            }
            let new_max = (cur_max + 50).min(200);
            sqlx::query("UPDATE users SET tokens=?, updated_at=NOW() WHERE id=?")
                .bind(tokens - 500)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "UPDATE clan_members SET max_stamina=?, stamina=?, updated_at=NOW() WHERE character_id=?",
            )
            .bind(new_max)
            .bind(new_max)
            .bind(char_id)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            Ok(json!("ok"))
        }
        ["player", "boost-prestige"] => {
            let mut tx = pool.begin().await?;
            let in_clan: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM clan_members WHERE character_id=?")
                    .bind(char_id)
                    .fetch_one(&mut *tx)
                    .await?;
            if in_clan == 0 {
                return Ok(json!({ "errorMessage": "Not in a clan" }));
            }
            let tokens = i64::from(
                sqlx::query_scalar::<_, i32>("SELECT tokens FROM users WHERE id=? FOR UPDATE")
                    .bind(user_id)
                    .fetch_one(&mut *tx)
                    .await?,
            );
            if tokens < 100 {
                return Ok(json!({ "errorMessage": "Not enough tokens" }));
            }
            let new_tokens = tokens - 100;
            sqlx::query("UPDATE users SET tokens=?, updated_at=NOW() WHERE id=?")
                .bind(new_tokens)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "UPDATE clan_members SET prestige_boost_expires_at = DATE_ADD(NOW(), INTERVAL 86400 SECOND), updated_at=NOW() WHERE character_id=?",
            )
            .bind(char_id)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            Ok(json!({
                "tokens": new_tokens,
                "prestige_boost": 86400,
                "result": "Prestige Boost activated for 24 hours!",
            }))
        }
        ["player", "stamina", "refill"] => {
            let Some(m) =
                sqlx::query("SELECT stamina, max_stamina FROM clan_members WHERE character_id=?")
                    .bind(char_id)
                    .fetch_optional(pool)
                    .await?
            else {
                return Ok(json!({ "errorMessage": "Not in a clan" }));
            };
            let st = m.get::<u32, _>("stamina");
            let max_st = m.get::<u32, _>("max_stamina");
            let new_st = (st + 50).min(max_st);
            sqlx::query("UPDATE clan_members SET stamina=?, updated_at=NOW() WHERE character_id=?")
                .bind(new_st)
                .bind(char_id)
                .execute(pool)
                .await?;
            Ok(json!("ok"))
        }
        ["battle", "defenders"] => {
            let Some(clan_id) = sqlx::query_scalar::<_, u64>(
                "SELECT clan_id FROM clan_members WHERE character_id=?",
            )
            .bind(char_id)
            .fetch_optional(pool)
            .await?
            else {
                return Ok(json!({ "defenders": [] }));
            };
            let members = format_clan_members(pool, clan_id).await?;
            Ok(json!({ "members": members }))
        }
        ["battle", "quick", opp_id_str] => {
            let opp_id = opp_id_str.parse::<u64>().unwrap_or(0);
            let mut tx = pool.begin().await?;
            let Some(m) = sqlx::query(
                "SELECT clan_id, stamina FROM clan_members WHERE character_id=? FOR UPDATE",
            )
            .bind(char_id)
            .fetch_optional(&mut *tx)
            .await?
            else {
                return Ok(json!({ "errorMessage": "Not in a clan" }));
            };
            let clan_id: u64 = m.get("clan_id");
            let stamina = i64::from(m.get::<u32, _>("stamina"));
            if stamina < 10 {
                return Ok(json!({ "errorMessage": "Not enough stamina" }));
            }
            let Some(def_row) =
                sqlx::query("SELECT name, prestige FROM clans WHERE id=? FOR UPDATE")
                    .bind(opp_id)
                    .fetch_optional(&mut *tx)
                    .await?
            else {
                return Ok(json!({ "errorMessage": "Clan not found" }));
            };
            let att_row = sqlx::query("SELECT prestige FROM clans WHERE id=? FOR UPDATE")
                .bind(clan_id)
                .fetch_one(&mut *tx)
                .await?;
            let att_rep = i64::from(att_row.get::<u32, _>("prestige"));
            let def_rep = i64::from(def_row.get::<u32, _>("prestige"));
            let def_name: String = def_row.get("name");
            let won = att_rep >= def_rep;
            let rep_gain = if won { 10 } else { 0 };
            let def_loss = if won { 10 } else { 0 };
            let prestige_gain = if won { 5 } else { 1 };
            let new_att_rep = att_rep + rep_gain;
            let new_def_rep = (def_rep - def_loss).max(0);
            let new_stamina = stamina - 10;

            sqlx::query("UPDATE clan_members SET stamina=?, updated_at=NOW() WHERE character_id=?")
                .bind(new_stamina)
                .bind(char_id)
                .execute(&mut *tx)
                .await?;
            if won {
                sqlx::query("UPDATE clans SET prestige=?, updated_at=NOW() WHERE id=?")
                    .bind(new_att_rep)
                    .bind(clan_id)
                    .execute(&mut *tx)
                    .await?;
                sqlx::query("UPDATE clans SET prestige=?, updated_at=NOW() WHERE id=?")
                    .bind(new_def_rep)
                    .bind(opp_id)
                    .execute(&mut *tx)
                    .await?;
            }
            tx.commit().await?;
            let (sid, _) = ensure_clan_season(pool).await?;
            sqlx::query(
                "INSERT INTO clan_battles (attacker_clan_id, defender_clan_id, season_id, attacker_won, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, NOW(), NOW())",
            )
            .bind(clan_id)
            .bind(opp_id)
            .bind(sid)
            .bind(if won { 1 } else { 0 })
            .execute(pool)
            .await?;

            Ok(json!({
                "reputation": new_att_rep,
                "gain": rep_gain,
                "stamina": new_stamina,
                "opponent_name": def_name,
                "opponent_reputation": new_def_rep,
                "prestige": prestige_gain,
            }))
        }
        ["battle", "manual", "start", opp_id_str] => {
            let opp_id = opp_id_str.parse::<u64>().unwrap_or(0);
            let Some(m) = sqlx::query("SELECT stamina FROM clan_members WHERE character_id=?")
                .bind(char_id)
                .fetch_optional(pool)
                .await?
            else {
                return Ok(json!({ "errorMessage": "Not in a clan" }));
            };
            if m.get::<u32, _>("stamina") < 10 {
                return Ok(json!({ "errorMessage": "Not enough stamina" }));
            }
            let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM clans WHERE id=?")
                .bind(opp_id)
                .fetch_one(pool)
                .await?;
            if exists == 0 {
                return Ok(json!({ "errorMessage": "Clan not found" }));
            }
            let enemies: Vec<u64> =
                sqlx::query_scalar("SELECT character_id FROM clan_members WHERE clan_id=? LIMIT 3")
                    .bind(opp_id)
                    .fetch_all(pool)
                    .await?;
            let token = auth::random_hex().unwrap_or_else(|_| "0123456789abcdef".into());
            Ok(json!({
                "battle_id": token,
                "enemies": enemies,
            }))
        }
        ["battle", "manual", "finish"] => {
            let opp_id = params
                .get("id")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            let mut tx = pool.begin().await?;
            let Some(m) = sqlx::query(
                "SELECT clan_id, stamina FROM clan_members WHERE character_id=? FOR UPDATE",
            )
            .bind(char_id)
            .fetch_optional(&mut *tx)
            .await?
            else {
                return Ok(json!({ "errorMessage": "Not in a clan" }));
            };
            let clan_id: u64 = m.get("clan_id");
            let stamina = i64::from(m.get::<u32, _>("stamina"));
            if stamina < 10 {
                return Ok(json!({ "errorMessage": "Not enough stamina" }));
            }
            let Some(def_row) =
                sqlx::query("SELECT name, prestige FROM clans WHERE id=? FOR UPDATE")
                    .bind(opp_id)
                    .fetch_optional(&mut *tx)
                    .await?
            else {
                return Ok(json!({ "errorMessage": "Opponent clan not found" }));
            };
            let att_row = sqlx::query("SELECT prestige FROM clans WHERE id=? FOR UPDATE")
                .bind(clan_id)
                .fetch_one(&mut *tx)
                .await?;
            let new_att_rep = i64::from(att_row.get::<u32, _>("prestige")) + 10;
            let new_def_rep = (i64::from(def_row.get::<u32, _>("prestige")) - 10).max(0);
            let def_name: String = def_row.get("name");
            let new_stamina = stamina - 10;

            sqlx::query("UPDATE clan_members SET stamina=?, updated_at=NOW() WHERE character_id=?")
                .bind(new_stamina)
                .bind(char_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE clans SET prestige=?, updated_at=NOW() WHERE id=?")
                .bind(new_att_rep)
                .bind(clan_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE clans SET prestige=?, updated_at=NOW() WHERE id=?")
                .bind(new_def_rep)
                .bind(opp_id)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;

            let (sid, _) = ensure_clan_season(pool).await?;
            sqlx::query(
                "INSERT INTO clan_battles (attacker_clan_id, defender_clan_id, season_id, attacker_won, created_at, updated_at) \
                 VALUES (?, ?, ?, 1, NOW(), NOW())",
            )
            .bind(clan_id)
            .bind(opp_id)
            .bind(sid)
            .execute(pool)
            .await?;

            Ok(json!({
                "reputation": new_att_rep,
                "gain": 10,
                "stamina": new_stamina,
                "opponent_name": def_name,
                "opponent_reputation": new_def_rep,
                "prestige": 5,
            }))
        }
        ["create"] => {
            let name = params.get("name").map(|s| s.trim()).unwrap_or("");
            if name.len() < 3 || name.len() > 50 {
                return Ok(json!({ "errorMessage": "Name must be 3-50 characters" }));
            }
            let in_clan: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM clan_members WHERE character_id=?")
                    .bind(char_id)
                    .fetch_one(pool)
                    .await?;
            if in_clan > 0 {
                return Ok(json!({ "errorMessage": "Already in a clan" }));
            }
            let taken: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM clans WHERE name=?")
                .bind(name)
                .fetch_one(pool)
                .await?;
            if taken > 0 {
                return Ok(json!({ "errorMessage": "Clan name already taken" }));
            }
            let (sid, _) = ensure_clan_season(pool).await?;
            let mut tx = pool.begin().await?;
            let res = sqlx::query(
                "INSERT INTO clans (season_id, name, master_id, max_members, buildings, created_at, updated_at) \
                 VALUES (?, ?, ?, 20, '{\"ramen\":0,\"hot_spring\":0,\"temple\":0,\"training_hall\":0}', NOW(), NOW())",
            )
            .bind(sid)
            .bind(name)
            .bind(char_id)
            .execute(&mut *tx)
            .await?;
            let clan_id = res.last_insert_id();
            sqlx::query(
                "INSERT INTO clan_members (clan_id, character_id, role, stamina, max_stamina, created_at, updated_at) \
                 VALUES (?, ?, 'master', 5, 5, NOW(), NOW())",
            )
            .bind(clan_id)
            .bind(char_id)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            Ok(json!("ok"))
        }
        ["rename"] => {
            let name = params.get("name").map(|s| s.trim()).unwrap_or("");
            if name.len() < 3 || name.len() > 50 {
                return Ok(json!({ "errorMessage": "Name must be 3-50 characters" }));
            }
            let Some(m) =
                sqlx::query("SELECT clan_id, role FROM clan_members WHERE character_id=?")
                    .bind(char_id)
                    .fetch_optional(pool)
                    .await?
            else {
                return Ok(json!({ "errorMessage": "Only master can rename clan" }));
            };
            if m.get::<String, _>("role") != "master" {
                return Ok(json!({ "errorMessage": "Only master can rename clan" }));
            }
            let clan_id: u64 = m.get("clan_id");
            let taken: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM clans WHERE name=? AND id!=?")
                    .bind(name)
                    .bind(clan_id)
                    .fetch_one(pool)
                    .await?;
            if taken > 0 {
                return Ok(json!({ "errorMessage": "Clan name already taken" }));
            }
            sqlx::query("UPDATE clans SET name=?, updated_at=NOW() WHERE id=?")
                .bind(name)
                .bind(clan_id)
                .execute(pool)
                .await?;
            Ok(json!("ok"))
        }
        ["player", "buy-onigiri", pkg_str] => {
            let pkg_id = pkg_str.parse::<i64>().unwrap_or(-1);
            let (qty, price) = match pkg_id {
                0 => (100i64, 975i64),
                1 => (200, 1900),
                2 => (500, 4625),
                _ => return Ok(json!({ "errorMessage": "Invalid package" })),
            };
            let mut tx = pool.begin().await?;
            let tokens = i64::from(
                sqlx::query_scalar::<_, i32>("SELECT tokens FROM users WHERE id=? FOR UPDATE")
                    .bind(user_id)
                    .fetch_one(&mut *tx)
                    .await?,
            );
            if tokens < price {
                return Ok(json!({ "errorMessage": "Not enough tokens" }));
            }
            sqlx::query("UPDATE users SET tokens=?, updated_at=NOW() WHERE id=?")
                .bind(tokens - price)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            rewards::give_item(&mut tx, char_id, "material_69", qty).await?;
            tx.commit().await?;
            Ok(json!({ "qty": qty, "price": price }))
        }
        ["member", _, "onigiri", "limit"] => Ok(json!({ "limit": 10, "used": 0 })),
        ["member", _, "onigiri", "gift", _] => Ok(json!({ "success": true })),
        ["invite-character", target_str] => {
            let target_id = target_str.parse::<i64>().unwrap_or(0);
            let Some(actor) =
                sqlx::query("SELECT clan_id, role FROM clan_members WHERE character_id=?")
                    .bind(char_id)
                    .fetch_optional(pool)
                    .await?
            else {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            };
            let role: String = actor.get("role");
            if role != "master" && role != "elder" {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            }
            let in_clan: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM clan_members WHERE character_id=?")
                    .bind(target_id)
                    .fetch_one(pool)
                    .await?;
            if in_clan > 0 {
                return Ok(json!({ "errorMessage": "Player is already in a clan" }));
            }
            let clan_id: u64 = actor.get("clan_id");
            let c_row = sqlx::query("SELECT name, max_members FROM clans WHERE id=?")
                .bind(clan_id)
                .fetch_one(pool)
                .await?;
            let cur_cnt: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM clan_members WHERE clan_id=?")
                    .bind(clan_id)
                    .fetch_one(pool)
                    .await?;
            if cur_cnt >= i64::from(c_row.get::<u32, _>("max_members")) {
                return Ok(json!({ "errorMessage": "Clan is full" }));
            }
            let c_name: String = c_row.get("name");
            sqlx::query(
                "INSERT INTO character_mails (character_id, type, title, sender, body, claimed, viewed, created_at, updated_at) \
                 VALUES (?, 'clan_invite', ?, ?, ?, 0, 0, NOW(), NOW())",
            )
            .bind(target_id)
            .bind(format!("{c_name} invites you to join!"))
            .bind(&c_name)
            .bind(serde_json::to_string(&json!({ "clan_id": clan_id })).unwrap_or_default())
            .execute(pool)
            .await?;
            Ok(json!({ "success": true }))
        }
        ["season-histories"] => {
            let rows = sqlx::query(
                "SELECT id, number, DATE_FORMAT(started_at, '%Y-%m-%dT%H:%i:%sZ') AS started_at, \
                 DATE_FORMAT(ended_at, '%Y-%m-%dT%H:%i:%sZ') AS ended_at \
                 FROM clan_seasons ORDER BY number DESC LIMIT 10",
            )
            .fetch_all(pool)
            .await?;
            let seasons: Vec<Json> = rows
                .into_iter()
                .map(|r| {
                    json!({
                        "id": r.get::<u64, _>("id"),
                        "number": r.get::<u32, _>("number"),
                        "started_at": r.get::<Option<String>, _>("started_at"),
                        "ended_at": r.get::<Option<String>, _>("ended_at"),
                    })
                })
                .collect();
            Ok(json!({ "seasons": seasons }))
        }
        _ => Ok(json!({ "errorMessage": format!("Unknown clan route: {path}") })),
    }
}
