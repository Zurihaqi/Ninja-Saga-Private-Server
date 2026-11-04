//! Administrative REST API (`/api/admin/*`) porting Filament resources, standalone
//! settings pages, season controllers, and `AdminCommandService` actions.

use crate::GameState;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use persistence::{MySqlPool, Row, row_json, sqlx};
use serde_json::{Value as JsonValue, json};
use std::collections::HashMap;

pub fn router() -> Router<GameState> {
    Router::new()
        .route("/api/admin/login", post(login_handler))
        .route("/api/admin/overview", get(overview_handler))
        .route(
            "/api/admin/resources/{resource}",
            get(list_resource_handler).post(create_resource_handler),
        )
        .route(
            "/api/admin/resources/{resource}/{id}",
            get(get_resource_handler)
                .put(update_resource_handler)
                .delete(delete_resource_handler),
        )
        .route("/api/admin/commands/execute", post(execute_command_handler))
        .route("/api/admin/configs", get(list_configs_handler))
        .route(
            "/api/admin/configs/{key}",
            get(get_config_handler).put(put_config_handler),
        )
        .route(
            "/api/admin/seasons/{kind}/{action}",
            post(season_action_handler),
        )
        .route("/api/admin/chat/messages", get(list_chat_handler))
        .route("/api/admin/chat/messages/{id}", delete(delete_chat_handler))
        .route("/api/admin/chat/announce", post(announce_chat_handler))
}

fn json_resp(status: StatusCode, body: JsonValue) -> Response {
    (
        status,
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        Json(body),
    )
        .into_response()
}

fn admin_secret() -> String {
    std::env::var("ADMIN_SECRET")
        .or_else(|_| std::env::var("CHAT_ADMIN_SECRET"))
        .unwrap_or_default()
}

async fn require_admin<'a>(
    state: &'a GameState,
    headers: &HeaderMap,
) -> Result<&'a MySqlPool, Response> {
    let Some(pool) = state.database.as_ref() else {
        return Err(json_resp(
            StatusCode::SERVICE_UNAVAILABLE,
            json!({ "status": 0, "error": "Database unavailable" }),
        ));
    };

    if let Some(secret_hdr) = headers.get("x-admin-secret").and_then(|v| v.to_str().ok()) {
        let expected = admin_secret();
        if !expected.is_empty() && secret_hdr == expected {
            return Ok(pool);
        }
    }

    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim)
        .unwrap_or("");

    if token.is_empty() {
        return Err(json_resp(
            StatusCode::UNAUTHORIZED,
            json!({ "status": 0, "error": "Unauthorized: admin credentials required" }),
        ));
    }

    let expected_secret = admin_secret();
    if !expected_secret.is_empty() && token == expected_secret {
        return Ok(pool);
    }

    let row = sqlx::query(
        "SELECT id, account_type, session_key FROM users WHERE session_key = ? LIMIT 1",
    )
    .bind(token)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if let Some(row) = row {
        let account_type = row
            .try_get::<i32, _>("account_type")
            .or_else(|_| row.try_get::<i8, _>("account_type").map(i32::from))
            .unwrap_or(0);
        if account_type == 99 {
            return Ok(pool);
        }
        return Err(json_resp(
            StatusCode::FORBIDDEN,
            json!({"status":0,"error":"Forbidden: account_type 99 required"}),
        ));
    }

    Err(json_resp(
        StatusCode::UNAUTHORIZED,
        json!({ "status": 0, "error": "Invalid admin credentials" }),
    ))
}

async fn login_handler(State(state): State<GameState>, Json(body): Json<JsonValue>) -> Response {
    let Some(pool) = state.database.as_ref() else {
        return json_resp(
            StatusCode::SERVICE_UNAVAILABLE,
            json!({ "status": 0, "error": "Database unavailable" }),
        );
    };

    if let Some(secret) = body.get("secret").and_then(JsonValue::as_str) {
        let expected = admin_secret();
        if !expected.is_empty() && secret == expected {
            return json_resp(
                StatusCode::OK,
                json!({
                    "status": 1,
                    "token": expected,
                    "user": {
                        "id": 0,
                        "username": "admin",
                        "name": "System Administrator",
                        "account_type": 99
                    }
                }),
            );
        }
    }

    let identifier = body
        .get("username")
        .or_else(|| body.get("email"))
        .and_then(JsonValue::as_str)
        .unwrap_or("")
        .trim();
    let password = body
        .get("password")
        .and_then(JsonValue::as_str)
        .unwrap_or("");

    if identifier.is_empty() || password.is_empty() {
        return json_resp(
            StatusCode::BAD_REQUEST,
            json!({ "status": 0, "error": "Username and password are required" }),
        );
    }

    let Ok(Some(row)) = sqlx::query(
        "SELECT id, username, name, email, password, account_type, session_key
         FROM users
         WHERE username = ? OR email = ?
         LIMIT 1",
    )
    .bind(identifier)
    .bind(identifier)
    .fetch_optional(pool)
    .await
    else {
        return json_resp(
            StatusCode::UNAUTHORIZED,
            json!({ "status": 0, "error": "Invalid credentials" }),
        );
    };

    let stored_hash: String = row.try_get("password").unwrap_or_default();
    let password = password.to_owned();
    let valid_pw = tokio::task::spawn_blocking(move || {
        authentication::verify_password(&password, &stored_hash)
    })
    .await
    .unwrap_or(false);
    if !valid_pw {
        return json_resp(
            StatusCode::UNAUTHORIZED,
            json!({ "status": 0, "error": "Invalid credentials" }),
        );
    }

    let account_type = row
        .try_get::<i32, _>("account_type")
        .or_else(|_| row.try_get::<i8, _>("account_type").map(i32::from))
        .unwrap_or(0);
    if account_type != 99 {
        return json_resp(
            StatusCode::FORBIDDEN,
            json!({ "status": 0, "error": "Admin access requires account_type 99" }),
        );
    }

    let user_id: u64 = row.try_get("id").unwrap_or(0);
    let existing_token: String = row
        .try_get::<Option<String>, _>("session_key")
        .ok()
        .flatten()
        .unwrap_or_default();
    let token = if existing_token.trim().is_empty() {
        let Ok(generated) = authentication::random_hex() else {
            return json_resp(
                StatusCode::SERVICE_UNAVAILABLE,
                json!({"status":0,"error":"Unable to create admin session"}),
            );
        };
        if sqlx::query("UPDATE users SET session_key = ? WHERE id = ?")
            .bind(&generated)
            .bind(user_id)
            .execute(pool)
            .await
            .is_err()
        {
            return json_resp(
                StatusCode::SERVICE_UNAVAILABLE,
                json!({"status":0,"error":"Unable to save admin session"}),
            );
        }
        generated
    } else {
        existing_token
    };

    json_resp(
        StatusCode::OK,
        json!({
            "status": 1,
            "token": token,
            "user": {
                "id": user_id,
                "username": row.try_get::<String, _>("username").unwrap_or_default(),
                "name": row.try_get::<String, _>("name").unwrap_or_default(),
                "email": row.try_get::<String, _>("email").unwrap_or_default(),
                "account_type": account_type,
            }
        }),
    )
}

async fn table_count(pool: &MySqlPool, table: &str) -> i64 {
    let sql = format!("SELECT COUNT(*) AS cnt FROM `{table}`");
    sqlx::query(&sql)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .and_then(|r| r.try_get::<i64, _>("cnt").ok())
        .unwrap_or(0)
}

async fn overview_handler(State(state): State<GameState>, headers: HeaderMap) -> Response {
    let pool = match require_admin(&state, &headers).await {
        Ok(p) => p,
        Err(r) => return r,
    };

    let mut counts = serde_json::Map::new();
    for table in [
        "users",
        "characters",
        "items",
        "skills",
        "pets",
        "missions",
        "enemies",
        "npcs",
        "talents",
        "senjutsus",
        "clans",
        "crews",
        "game_events",
        "giveaways",
        "character_mails",
        "pvp_battles",
        "admin_commands",
        "mysterious_markets",
        "x_p_s",
    ] {
        counts.insert(table.to_owned(), json!(table_count(pool, table).await));
    }

    let clan_season = sqlx::query(
        "SELECT id, number, active, CAST(started_at AS CHAR) AS started_at, CAST(ended_at AS CHAR) AS ended_at
         FROM clan_seasons ORDER BY id DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .and_then(|r| row_json(&r).ok());

    let crew_season = sqlx::query(
        "SELECT id, number, active, phase, CAST(started_at AS CHAR) AS started_at, CAST(ended_at AS CHAR) AS ended_at
         FROM crew_seasons ORDER BY id DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .and_then(|r| row_json(&r).ok());

    let sw_season = sqlx::query(
        "SELECT id, num, active, CAST(start_at AS CHAR) AS start_at, CAST(end_at AS CHAR) AS end_at
         FROM shadow_war_seasons ORDER BY id DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .and_then(|r| row_json(&r).ok());

    json_resp(
        StatusCode::OK,
        json!({
            "status": 1,
            "counts": counts,
            "seasons": {
                "clan": clan_season,
                "crew": crew_season,
                "shadow_war": sw_season,
            }
        }),
    )
}

fn resolve_table(resource: &str) -> Option<&'static str> {
    match resource {
        "users" => Some("users"),
        "characters" => Some("characters"),
        "character_items" => Some("character_items"),
        "character_skills" => Some("character_skills"),
        "character_pets" => Some("character_pets"),
        "items" => Some("items"),
        "skills" => Some("skills"),
        "pets" => Some("pets"),
        "missions" => Some("missions"),
        "enemies" => Some("enemies"),
        "npcs" => Some("npcs"),
        "talents" => Some("talents"),
        "senjutsus" => Some("senjutsus"),
        "xp_levels" | "x_p_s" => Some("x_p_s"),
        "game_events" | "events" => Some("game_events"),
        "event_analytics" | "character_event_data" => Some("character_event_data"),
        "giveaways" => Some("giveaways"),
        "mails" | "character_mails" => Some("character_mails"),
        "mysterious_markets" => Some("mysterious_markets"),
        "admin_commands" => Some("admin_commands"),
        "clan_seasons" => Some("clan_seasons"),
        "crew_seasons" => Some("crew_seasons"),
        "shadow_war_seasons" => Some("shadow_war_seasons"),
        "clans" => Some("clans"),
        "crews" => Some("crews"),
        "pvp_battles" => Some("pvp_battles"),
        _ => None,
    }
}

fn sanitize_column(col: &str) -> bool {
    !col.is_empty()
        && col.len() <= 64
        && col.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

async fn list_resource_handler(
    State(state): State<GameState>,
    headers: HeaderMap,
    Path(resource): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let pool = match require_admin(&state, &headers).await {
        Ok(p) => p,
        Err(r) => return r,
    };

    if resource == "event_analytics" && !params.contains_key("character_id") {
        let rows = sqlx::query(
            "SELECT event_key,
                    COUNT(*) AS player_count,
                    CAST( COALESCE(SUM(battles), 0) AS SIGNED ) AS total_battles,
                    CAST( COALESCE(SUM(bought), 0) AS SIGNED ) AS bought_count,
                    CAST( COALESCE(SUM(energy), 0) AS SIGNED ) AS total_energy_remaining
             FROM character_event_data
             GROUP BY event_key
             ORDER BY event_key ASC",
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default();
        let data: Vec<JsonValue> = rows.iter().filter_map(|r| row_json(r).ok()).collect();
        return json_resp(StatusCode::OK, json!({ "status": 1, "data": data }));
    }

    let Some(table) = resolve_table(&resource) else {
        return json_resp(
            StatusCode::NOT_FOUND,
            json!({ "status": 0, "error": format!("Unknown resource: {resource}") }),
        );
    };

    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(100)
        .clamp(1, 500);
    let offset = params
        .get("offset")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0)
        .max(0);

    let mut sql = format!("SELECT * FROM `{table}` WHERE 1=1");
    let mut binds: Vec<String> = Vec::new();

    if let Some(cid) = params.get("character_id") {
        sql.push_str(" AND `character_id` = ?");
        binds.push(cid.clone());
    }
    if let Some(cat) = params.get("category") {
        sql.push_str(" AND `category` = ?");
        binds.push(cat.clone());
    }
    if let Some(search) = params
        .get("search")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        let pattern = format!("%{search}%");
        match table {
            "users" => {
                sql.push_str(" AND (`username` LIKE ? OR `email` LIKE ? OR `name` LIKE ?)");
                binds.extend([pattern.clone(), pattern.clone(), pattern]);
            }
            "characters" | "clans" | "crews" | "npcs" | "enemies" | "pets" | "skills"
            | "talents" | "senjutsus" | "admin_commands" => {
                sql.push_str(" AND `name` LIKE ?");
                binds.push(pattern);
            }
            "items" => {
                sql.push_str(" AND (`item_id` LIKE ? OR `name` LIKE ?)");
                binds.extend([pattern.clone(), pattern]);
            }
            "missions" => {
                sql.push_str(" AND `mission_id` LIKE ?");
                binds.push(pattern);
            }
            "game_events" | "giveaways" | "character_mails" => {
                sql.push_str(" AND `title` LIKE ?");
                binds.push(pattern);
            }
            _ => {}
        }
    }

    sql.push_str(" ORDER BY `id` DESC LIMIT ? OFFSET ?");

    let mut q = sqlx::query(&sql);
    for b in &binds {
        q = q.bind(b);
    }
    q = q.bind(limit).bind(offset);

    let rows = match q.fetch_all(pool).await {
        Ok(r) => r,
        Err(err) => {
            return json_resp(
                StatusCode::BAD_REQUEST,
                json!({ "status": 0, "error": err.to_string() }),
            );
        }
    };

    let mut data = Vec::with_capacity(rows.len());
    for row in &rows {
        if let Ok(mut obj) = row_json(row) {
            if table == "users"
                && let Some(map) = obj.as_object_mut()
            {
                map.remove("password");
            }
            data.push(obj);
        }
    }

    json_resp(
        StatusCode::OK,
        json!({
            "status": 1,
            "table": table,
            "data": data,
        }),
    )
}

async fn get_resource_handler(
    State(state): State<GameState>,
    headers: HeaderMap,
    Path((resource, id)): Path<(String, i64)>,
) -> Response {
    let pool = match require_admin(&state, &headers).await {
        Ok(p) => p,
        Err(r) => return r,
    };
    let Some(table) = resolve_table(&resource) else {
        return json_resp(
            StatusCode::NOT_FOUND,
            json!({ "status": 0, "error": format!("Unknown resource: {resource}") }),
        );
    };

    let sql = format!("SELECT * FROM `{table}` WHERE `id` = ? LIMIT 1");
    let Ok(Some(row)) = sqlx::query(&sql).bind(id).fetch_optional(pool).await else {
        return json_resp(
            StatusCode::NOT_FOUND,
            json!({ "status": 0, "error": "Record not found" }),
        );
    };
    let mut obj = row_json(&row).unwrap_or_else(|_| json!({}));
    if table == "users"
        && let Some(map) = obj.as_object_mut()
    {
        map.remove("password");
    }

    json_resp(StatusCode::OK, json!({ "status": 1, "data": obj }))
}

fn bind_json_value<'q>(
    q: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
    val: &'q JsonValue,
) -> sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments> {
    match val {
        JsonValue::Null => q.bind(Option::<String>::None),
        JsonValue::Bool(b) => q.bind(i32::from(*b)),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                q.bind(i)
            } else if let Some(u) = n.as_u64() {
                q.bind(u)
            } else {
                q.bind(n.as_f64().unwrap_or(0.0))
            }
        }
        JsonValue::String(s) => q.bind(s.as_str()),
        JsonValue::Array(_) | JsonValue::Object(_) => q.bind(val.to_string()),
    }
}

async fn create_resource_handler(
    State(state): State<GameState>,
    headers: HeaderMap,
    Path(resource): Path<String>,
    Json(mut body): Json<JsonValue>,
) -> Response {
    let pool = match require_admin(&state, &headers).await {
        Ok(p) => p,
        Err(r) => return r,
    };
    let Some(table) = resolve_table(&resource) else {
        return json_resp(
            StatusCode::NOT_FOUND,
            json!({ "status": 0, "error": format!("Unknown resource: {resource}") }),
        );
    };

    let Some(map) = body.as_object_mut() else {
        return json_resp(
            StatusCode::BAD_REQUEST,
            json!({ "status": 0, "error": "Body must be a JSON object" }),
        );
    };
    map.remove("id");
    map.remove("created_at");
    map.remove("updated_at");

    if table == "users"
        && let Some(pw) = map.get("password").and_then(JsonValue::as_str)
        && !pw.starts_with("$2")
        && let Ok(hashed) = authentication::hash_password(pw)
    {
        map.insert("password".to_owned(), json!(hashed));
    }

    let mut cols = Vec::new();
    let mut vals = Vec::new();
    for (k, v) in map.iter() {
        if !sanitize_column(k) {
            return json_resp(
                StatusCode::BAD_REQUEST,
                json!({ "status": 0, "error": format!("Invalid column name: {k}") }),
            );
        }
        cols.push(format!("`{k}`"));
        vals.push(v.clone());
    }
    if cols.is_empty() {
        return json_resp(
            StatusCode::BAD_REQUEST,
            json!({ "status": 0, "error": "No fields provided" }),
        );
    }

    let placeholders = vec!["?"; cols.len()].join(", ");
    let sql = format!(
        "INSERT INTO `{table}` ({}) VALUES ({placeholders})",
        cols.join(", ")
    );
    let mut q = sqlx::query(&sql);
    for v in &vals {
        q = bind_json_value(q, v);
    }

    match q.execute(pool).await {
        Ok(res) => json_resp(
            StatusCode::CREATED,
            json!({
                "status": 1,
                "id": res.last_insert_id(),
            }),
        ),
        Err(err) => json_resp(
            StatusCode::BAD_REQUEST,
            json!({ "status": 0, "error": err.to_string() }),
        ),
    }
}

async fn update_resource_handler(
    State(state): State<GameState>,
    headers: HeaderMap,
    Path((resource, id)): Path<(String, i64)>,
    Json(mut body): Json<JsonValue>,
) -> Response {
    let pool = match require_admin(&state, &headers).await {
        Ok(p) => p,
        Err(r) => return r,
    };
    let Some(table) = resolve_table(&resource) else {
        return json_resp(
            StatusCode::NOT_FOUND,
            json!({ "status": 0, "error": format!("Unknown resource: {resource}") }),
        );
    };

    let Some(map) = body.as_object_mut() else {
        return json_resp(
            StatusCode::BAD_REQUEST,
            json!({ "status": 0, "error": "Body must be a JSON object" }),
        );
    };
    map.remove("id");
    map.remove("created_at");
    map.remove("updated_at");

    if table == "users"
        && let Some(pw) = map.get("password").and_then(JsonValue::as_str)
    {
        if pw.trim().is_empty() {
            map.remove("password");
        } else if !pw.starts_with("$2")
            && let Ok(hashed) = authentication::hash_password(pw)
        {
            map.insert("password".to_owned(), json!(hashed));
        }
    }

    let mut sets = Vec::new();
    let mut vals = Vec::new();
    for (k, v) in map.iter() {
        if !sanitize_column(k) {
            return json_resp(
                StatusCode::BAD_REQUEST,
                json!({ "status": 0, "error": format!("Invalid column name: {k}") }),
            );
        }
        sets.push(format!("`{k}` = ?"));
        vals.push(v.clone());
    }
    if sets.is_empty() {
        return json_resp(
            StatusCode::BAD_REQUEST,
            json!({ "status": 0, "error": "No fields provided" }),
        );
    }

    let sql = format!("UPDATE `{table}` SET {} WHERE `id` = ?", sets.join(", "));
    let mut q = sqlx::query(&sql);
    for v in &vals {
        q = bind_json_value(q, v);
    }
    q = q.bind(id);

    match q.execute(pool).await {
        Ok(res) => json_resp(
            StatusCode::OK,
            json!({
                "status": 1,
                "affected": res.rows_affected(),
            }),
        ),
        Err(err) => json_resp(
            StatusCode::BAD_REQUEST,
            json!({ "status": 0, "error": err.to_string() }),
        ),
    }
}

async fn delete_resource_handler(
    State(state): State<GameState>,
    headers: HeaderMap,
    Path((resource, id)): Path<(String, i64)>,
) -> Response {
    let pool = match require_admin(&state, &headers).await {
        Ok(p) => p,
        Err(r) => return r,
    };
    let Some(table) = resolve_table(&resource) else {
        return json_resp(
            StatusCode::NOT_FOUND,
            json!({ "status": 0, "error": format!("Unknown resource: {resource}") }),
        );
    };

    let sql = format!("DELETE FROM `{table}` WHERE `id` = ?");
    match sqlx::query(&sql).bind(id).execute(pool).await {
        Ok(res) => json_resp(
            StatusCode::OK,
            json!({
                "status": 1,
                "deleted": res.rows_affected(),
            }),
        ),
        Err(err) => json_resp(
            StatusCode::BAD_REQUEST,
            json!({ "status": 0, "error": err.to_string() }),
        ),
    }
}

async fn grant_category_items(
    pool: &MySqlPool,
    character_id: i64,
    char_name: &str,
    category: &str,
    label: &str,
) -> (bool, String) {
    let rows = sqlx::query("SELECT item_id FROM items WHERE category = ?")
        .bind(category)
        .fetch_all(pool)
        .await
        .unwrap_or_default();
    let mut count = 0;
    for row in rows {
        let item_id: String = row.try_get("item_id").unwrap_or_default();
        if item_id.is_empty() {
            continue;
        }
        let exists = sqlx::query(
            "SELECT id FROM character_items WHERE character_id = ? AND item_id = ? LIMIT 1",
        )
        .bind(character_id)
        .bind(&item_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .is_some();
        if !exists
            && sqlx::query(
                "INSERT INTO character_items (character_id, item_id, quantity, category, created_at, updated_at)
                 VALUES (?, ?, 1, ?, NOW(), NOW())",
            )
            .bind(character_id)
            .bind(&item_id)
            .bind(category)
            .execute(pool)
            .await
            .is_ok()
        {
            count += 1;
        }
    }
    (true, format!("Granted {count} new {label} to {char_name}."))
}

async fn run_command_type(
    pool: &MySqlPool,
    character_id: i64,
    user_id: i64,
    char_name: &str,
    command_type: &str,
    params: &JsonValue,
) -> (bool, String) {
    match command_type {
        "give_all_skills" => {
            let rows = sqlx::query("SELECT skill_id FROM skills")
                .fetch_all(pool)
                .await
                .unwrap_or_default();
            let mut count = 0;
            for row in rows {
                let skill_id: String = row.try_get("skill_id").unwrap_or_default();
                if skill_id.is_empty() {
                    continue;
                }
                let exists = sqlx::query(
                    "SELECT id FROM character_skills WHERE character_id = ? AND skill_id = ? LIMIT 1",
                )
                .bind(character_id)
                .bind(&skill_id)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
                .is_some();
                if !exists
                    && sqlx::query(
                        "INSERT INTO character_skills (character_id, skill_id, created_at, updated_at)
                         VALUES (?, ?, NOW(), NOW())",
                    )
                    .bind(character_id)
                    .bind(&skill_id)
                    .execute(pool)
                    .await
                    .is_ok()
                {
                    count += 1;
                }
            }
            (true, format!("Granted {count} new skills to {char_name}."))
        }
        "give_all_pets" => {
            let rows = sqlx::query("SELECT pet_id, name FROM pets")
                .fetch_all(pool)
                .await
                .unwrap_or_default();
            let mut count = 0;
            for row in rows {
                let pet_id: String = row.try_get("pet_id").unwrap_or_default();
                let pet_name: String = row.try_get("name").unwrap_or_else(|_| pet_id.clone());
                if pet_id.is_empty() {
                    continue;
                }
                let exists = sqlx::query(
                    "SELECT id FROM character_pets WHERE character_id = ? AND pet_id = ? LIMIT 1",
                )
                .bind(character_id)
                .bind(&pet_id)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
                .is_some();
                if !exists
                    && sqlx::query(
                        "INSERT INTO character_pets (character_id, pet_id, name, level, xp, maturity_points, created_at, updated_at)
                         VALUES (?, ?, ?, 1, 0, 0, NOW(), NOW())",
                    )
                    .bind(character_id)
                    .bind(&pet_id)
                    .bind(&pet_name)
                    .execute(pool)
                    .await
                    .is_ok()
                {
                    count += 1;
                }
            }
            (true, format!("Granted {count} new pets to {char_name}."))
        }
        "give_all_category" => {
            let cat = params
                .get("category")
                .and_then(JsonValue::as_str)
                .unwrap_or("weapon");
            grant_category_items(pool, character_id, char_name, cat, &format!("{cat} items")).await
        }
        "give_all_hairstyles" => {
            grant_category_items(pool, character_id, char_name, "hair", "hairstyles").await
        }
        "give_all_weapons" => {
            grant_category_items(pool, character_id, char_name, "weapon", "weapons").await
        }
        "give_all_spendingitems" => {
            grant_category_items(pool, character_id, char_name, "spending", "spending items").await
        }
        "give_all_shadowwaritems" => {
            grant_category_items(
                pool,
                character_id,
                char_name,
                "shadowwar",
                "shadowwar items",
            )
            .await
        }
        "give_all_setitems" => {
            grant_category_items(pool, character_id, char_name, "set", "set items").await
        }
        "give_all_packageitems" => {
            grant_category_items(pool, character_id, char_name, "package", "package items").await
        }
        "give_all_materialitems" => {
            grant_category_items(pool, character_id, char_name, "material", "material items").await
        }
        "give_all_leaderboarditems" => {
            grant_category_items(
                pool,
                character_id,
                char_name,
                "leaderboard",
                "leaderboard items",
            )
            .await
        }
        "give_all_itemitems" => {
            grant_category_items(
                pool,
                character_id,
                char_name,
                "item",
                "item (Category Item) items",
            )
            .await
        }
        "give_all_eventitems" => {
            grant_category_items(pool, character_id, char_name, "event", "event items").await
        }
        "give_all_essentialitems" => {
            grant_category_items(
                pool,
                character_id,
                char_name,
                "essential",
                "essential items",
            )
            .await
        }
        "give_all_dealitems" => {
            grant_category_items(pool, character_id, char_name, "deal", "deal items").await
        }
        "give_all_crewitems" => {
            grant_category_items(pool, character_id, char_name, "crew", "crew items").await
        }
        "give_all_clanitems" => {
            grant_category_items(pool, character_id, char_name, "clan", "clan items").await
        }
        "give_all_accessoryitems" => {
            grant_category_items(
                pool,
                character_id,
                char_name,
                "accessory",
                "accessory items",
            )
            .await
        }
        "give_all_backitems" => {
            grant_category_items(pool, character_id, char_name, "back", "backitems").await
        }
        "give_all_available" => {
            let mut messages = Vec::new();
            for sub in [
                "give_all_skills",
                "give_all_hairstyles",
                "give_all_pets",
                "give_all_crewitems",
                "give_all_clanitems",
                "give_all_weapons",
                "give_all_spendingitems",
                "give_all_shadowwaritems",
                "give_all_setitems",
                "give_all_packageitems",
                "give_all_materialitems",
                "give_all_leaderboarditems",
                "give_all_itemitems",
                "give_all_eventitems",
                "give_all_essentialitems",
                "give_all_dealitems",
                "give_all_accessoryitems",
                "give_all_backitems",
            ] {
                let (_, msg) = Box::pin(run_command_type(
                    pool,
                    character_id,
                    user_id,
                    char_name,
                    sub,
                    params,
                ))
                .await;
                messages.push(msg);
            }
            (
                true,
                format!(
                    "All available items granted to {char_name}: {}",
                    messages.join(" | ")
                ),
            )
        }
        "add_gold" => {
            let amount = params
                .get("amount")
                .and_then(|v| {
                    v.as_i64()
                        .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
                })
                .unwrap_or(0);
            let _ = sqlx::query("UPDATE characters SET gold = gold + ? WHERE id = ?")
                .bind(amount)
                .bind(character_id)
                .execute(pool)
                .await;
            let new_gold: i64 = sqlx::query("SELECT gold FROM characters WHERE id = ? LIMIT 1")
                .bind(character_id)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
                .and_then(|r| {
                    r.try_get::<i64, _>("gold")
                        .or_else(|_| r.try_get::<i32, _>("gold").map(i64::from))
                        .ok()
                })
                .unwrap_or(0);
            (
                true,
                format!("Added {amount} gold to {char_name}. New total: {new_gold}."),
            )
        }
        "add_tokens" => {
            let amount = params
                .get("amount")
                .and_then(|v| {
                    v.as_i64()
                        .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
                })
                .unwrap_or(0);
            let _ = sqlx::query("UPDATE users SET tokens = tokens + ? WHERE id = ?")
                .bind(amount)
                .bind(user_id)
                .execute(pool)
                .await;
            (
                true,
                format!("Added {amount} tokens to {char_name}'s account."),
            )
        }
        "set_rank" => {
            let rank = params
                .get("rank")
                .and_then(|v| {
                    v.as_i64()
                        .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
                })
                .unwrap_or(1)
                .clamp(1, 10);
            let _ = sqlx::query("UPDATE characters SET `rank` = ? WHERE id = ?")
                .bind(rank)
                .bind(character_id)
                .execute(pool)
                .await;
            (true, format!("Set {char_name}'s rank to {rank}."))
        }
        "set_level" => {
            let level = params
                .get("level")
                .and_then(|v| {
                    v.as_i64()
                        .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
                })
                .unwrap_or(1)
                .clamp(1, 100);
            let _ = sqlx::query("UPDATE characters SET level = ? WHERE id = ?")
                .bind(level)
                .bind(character_id)
                .execute(pool)
                .await;
            (true, format!("Set {char_name}'s level to {level}."))
        }
        other => (false, format!("Unknown command type: {other}")),
    }
}

async fn execute_command_handler(
    State(state): State<GameState>,
    headers: HeaderMap,
    Json(body): Json<JsonValue>,
) -> Response {
    let pool = match require_admin(&state, &headers).await {
        Ok(p) => p,
        Err(r) => return r,
    };

    let character_id = body
        .get("character_id")
        .and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
        })
        .unwrap_or(0);
    if character_id <= 0 {
        return json_resp(
            StatusCode::BAD_REQUEST,
            json!({ "status": 0, "error": "character_id is required" }),
        );
    }

    let Ok(Some(char_row)) =
        sqlx::query("SELECT id, user_id, name FROM characters WHERE id = ? LIMIT 1")
            .bind(character_id)
            .fetch_optional(pool)
            .await
    else {
        return json_resp(
            StatusCode::NOT_FOUND,
            json!({ "status": 0, "error": "Character not found" }),
        );
    };

    let user_id: i64 = char_row
        .try_get::<i64, _>("user_id")
        .or_else(|_| char_row.try_get::<u64, _>("user_id").map(|v| v as i64))
        .unwrap_or(0);
    let char_name: String = char_row.try_get("name").unwrap_or_default();

    let mut command_type = body
        .get("command_type")
        .and_then(JsonValue::as_str)
        .unwrap_or("")
        .to_owned();
    let mut merged_params = body.get("params").cloned().unwrap_or_else(|| json!({}));

    if let Some(cmd_id) = body.get("command_id").and_then(|v| {
        v.as_i64()
            .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
    }) {
        let Ok(Some(cmd_row)) =
            sqlx::query("SELECT command_type, params FROM admin_commands WHERE id = ? LIMIT 1")
                .bind(cmd_id)
                .fetch_optional(pool)
                .await
        else {
            return json_resp(
                StatusCode::NOT_FOUND,
                json!({ "status": 0, "error": "AdminCommand not found" }),
            );
        };
        command_type = cmd_row.try_get("command_type").unwrap_or_default();
        if let Ok(Some(mut db_params)) = cmd_row.try_get::<Option<JsonValue>, _>("params")
            && let (Some(base), Some(override_map)) =
                (db_params.as_object_mut(), merged_params.as_object())
        {
            for (k, v) in override_map {
                base.insert(k.clone(), v.clone());
            }
            merged_params = JsonValue::Object(base.clone());
        }
    }

    let (success, message) = run_command_type(
        pool,
        character_id,
        user_id,
        &char_name,
        &command_type,
        &merged_params,
    )
    .await;

    json_resp(
        if success {
            StatusCode::OK
        } else {
            StatusCode::BAD_REQUEST
        },
        json!({
            "status": i32::from(success),
            "success": success,
            "message": message,
        }),
    )
}

async fn list_configs_handler(State(state): State<GameState>, headers: HeaderMap) -> Response {
    let pool = match require_admin(&state, &headers).await {
        Ok(p) => p,
        Err(r) => return r,
    };
    let rows = sqlx::query("SELECT id, `key`, `value` FROM game_configs ORDER BY `key` ASC")
        .fetch_all(pool)
        .await
        .unwrap_or_default();
    let mut configs = serde_json::Map::new();
    for row in rows {
        let key: String = row.try_get("key").unwrap_or_default();
        let val: JsonValue = row
            .try_get::<Option<JsonValue>, _>("value")
            .ok()
            .flatten()
            .unwrap_or(JsonValue::Null);
        configs.insert(key, val);
    }
    json_resp(StatusCode::OK, json!({ "status": 1, "configs": configs }))
}

async fn get_config_handler(
    State(state): State<GameState>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> Response {
    let pool = match require_admin(&state, &headers).await {
        Ok(p) => p,
        Err(r) => return r,
    };
    let row = sqlx::query("SELECT `value` FROM game_configs WHERE `key` = ? LIMIT 1")
        .bind(&key)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();
    let value = row
        .and_then(|r| r.try_get::<Option<JsonValue>, _>("value").ok().flatten())
        .unwrap_or_else(|| json!({}));
    json_resp(
        StatusCode::OK,
        json!({
            "status": 1,
            "key": key,
            "value": value,
        }),
    )
}

async fn put_config_handler(
    State(state): State<GameState>,
    headers: HeaderMap,
    Path(key): Path<String>,
    Json(body): Json<JsonValue>,
) -> Response {
    let pool = match require_admin(&state, &headers).await {
        Ok(p) => p,
        Err(r) => return r,
    };
    let value = body.get("value").cloned().unwrap_or(body);
    let serialized = value.to_string();
    let res = sqlx::query(
        "INSERT INTO game_configs (`key`, `value`, created_at, updated_at)
         VALUES (?, ?, NOW(), NOW())
         ON DUPLICATE KEY UPDATE `value` = VALUES(`value`), updated_at = NOW()",
    )
    .bind(&key)
    .bind(&serialized)
    .execute(pool)
    .await;

    match res {
        Ok(_) => json_resp(
            StatusCode::OK,
            json!({
                "status": 1,
                "key": key,
                "value": value,
            }),
        ),
        Err(err) => json_resp(
            StatusCode::BAD_REQUEST,
            json!({ "status": 0, "error": err.to_string() }),
        ),
    }
}

async fn season_action_handler(
    State(state): State<GameState>,
    headers: HeaderMap,
    Path((kind, action)): Path<(String, String)>,
    Json(body): Json<JsonValue>,
) -> Response {
    let pool = match require_admin(&state, &headers).await {
        Ok(p) => p,
        Err(r) => return r,
    };
    let duration_days = body
        .get("duration_days")
        .and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
        })
        .unwrap_or(30)
        .clamp(1, 365);

    match (kind.as_str(), action.as_str()) {
        ("clan", "start") | ("clan", "end_and_advance") => {
            let _ = sqlx::query(
                "UPDATE clan_seasons SET active = 0, ended_at = NOW() WHERE active = 1",
            )
            .execute(pool)
            .await;
            let next_num: i64 =
                sqlx::query("SELECT COALESCE(MAX(number), 0) + 1 AS n FROM clan_seasons")
                    .fetch_one(pool)
                    .await
                    .ok()
                    .and_then(|r| r.try_get("n").ok())
                    .unwrap_or(1);
            let res = sqlx::query(
                "INSERT INTO clan_seasons (number, active, started_at, ended_at, created_at, updated_at)
                 VALUES (?, 1, NOW(), DATE_ADD(NOW(), INTERVAL ? DAY), NOW(), NOW())",
            )
            .bind(next_num)
            .bind(duration_days)
            .execute(pool)
            .await;
            match res {
                Ok(r) => json_resp(
                    StatusCode::OK,
                    json!({ "status": 1, "season_id": r.last_insert_id(), "number": next_num }),
                ),
                Err(e) => json_resp(
                    StatusCode::BAD_REQUEST,
                    json!({ "status": 0, "error": e.to_string() }),
                ),
            }
        }
        ("crew", "start") | ("crew", "end_and_advance") => {
            let _ = sqlx::query(
                "UPDATE crew_seasons SET active = 0, ended_at = NOW() WHERE active = 1",
            )
            .execute(pool)
            .await;
            let next_num: i64 =
                sqlx::query("SELECT COALESCE(MAX(number), 0) + 1 AS n FROM crew_seasons")
                    .fetch_one(pool)
                    .await
                    .ok()
                    .and_then(|r| r.try_get("n").ok())
                    .unwrap_or(1);
            let res = sqlx::query(
                "INSERT INTO crew_seasons (number, active, phase, started_at, ended_at, phase_ends_at, created_at, updated_at)
                 VALUES (?, 1, 1, NOW(), DATE_ADD(NOW(), INTERVAL ? DAY), DATE_ADD(NOW(), INTERVAL 15 DAY), NOW(), NOW())",
            )
            .bind(next_num)
            .bind(duration_days)
            .execute(pool)
            .await;
            match res {
                Ok(r) => json_resp(
                    StatusCode::OK,
                    json!({ "status": 1, "season_id": r.last_insert_id(), "number": next_num }),
                ),
                Err(e) => json_resp(
                    StatusCode::BAD_REQUEST,
                    json!({ "status": 0, "error": e.to_string() }),
                ),
            }
        }
        ("crew", "advance_phase") => {
            let _ = sqlx::query(
                "UPDATE crew_seasons SET phase = CASE WHEN phase = 1 THEN 2 ELSE 1 END WHERE active = 1",
            )
            .execute(pool)
            .await;
            json_resp(StatusCode::OK, json!({ "status": 1 }))
        }
        ("shadow_war", "start") | ("shadow_war", "end_and_advance") => {
            let _ = sqlx::query(
                "UPDATE shadow_war_seasons SET active = 0, end_at = NOW() WHERE active = 1",
            )
            .execute(pool)
            .await;
            let next_num: i64 =
                sqlx::query("SELECT COALESCE(MAX(num), 0) + 1 AS n FROM shadow_war_seasons")
                    .fetch_one(pool)
                    .await
                    .ok()
                    .and_then(|r| r.try_get("n").ok())
                    .unwrap_or(1);
            let res = sqlx::query(
                "INSERT INTO shadow_war_seasons (num, active, start_at, end_at, created_at, updated_at)
                 VALUES (?, 1, NOW(), DATE_ADD(NOW(), INTERVAL ? DAY), NOW(), NOW())",
            )
            .bind(next_num)
            .bind(duration_days)
            .execute(pool)
            .await;
            match res {
                Ok(r) => json_resp(
                    StatusCode::OK,
                    json!({ "status": 1, "season_id": r.last_insert_id(), "num": next_num }),
                ),
                Err(e) => json_resp(
                    StatusCode::BAD_REQUEST,
                    json!({ "status": 0, "error": e.to_string() }),
                ),
            }
        }
        _ => json_resp(
            StatusCode::BAD_REQUEST,
            json!({ "status": 0, "error": "Unsupported season action" }),
        ),
    }
}

async fn ensure_chat_table(pool: &MySqlPool) {
    let _ = sqlx::query(
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
    .await;
}

async fn list_chat_handler(
    State(state): State<GameState>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let pool = match require_admin(&state, &headers).await {
        Ok(p) => p,
        Err(r) => return r,
    };
    ensure_chat_table(pool).await;
    let channel = params
        .get("channel")
        .map(String::as_str)
        .unwrap_or("global");
    let rows = sqlx::query(
        "SELECT id, channel, character_id, character_name, message, timestamp
         FROM chat_messages
         WHERE channel = ?
         ORDER BY id DESC
         LIMIT 100",
    )
    .bind(channel)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    let data: Vec<JsonValue> = rows.iter().filter_map(|r| row_json(r).ok()).collect();
    json_resp(StatusCode::OK, json!({ "status": 1, "data": data }))
}

async fn delete_chat_handler(
    State(state): State<GameState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    let pool = match require_admin(&state, &headers).await {
        Ok(p) => p,
        Err(r) => return r,
    };
    ensure_chat_table(pool).await;
    let res = sqlx::query("DELETE FROM chat_messages WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await;
    match res {
        Ok(r) => json_resp(
            StatusCode::OK,
            json!({ "status": 1, "deleted": r.rows_affected() }),
        ),
        Err(e) => json_resp(
            StatusCode::BAD_REQUEST,
            json!({ "status": 0, "error": e.to_string() }),
        ),
    }
}

async fn announce_chat_handler(
    State(state): State<GameState>,
    headers: HeaderMap,
    Json(body): Json<JsonValue>,
) -> Response {
    let pool = match require_admin(&state, &headers).await {
        Ok(p) => p,
        Err(r) => return r,
    };
    ensure_chat_table(pool).await;
    let message: String = body
        .get("message")
        .and_then(JsonValue::as_str)
        .unwrap_or("")
        .trim()
        .chars()
        .take(500)
        .collect();
    if message.is_empty() {
        return json_resp(
            StatusCode::BAD_REQUEST,
            json!({ "status": 0, "error": "Message cannot be empty" }),
        );
    }
    let channel = body
        .get("channel")
        .and_then(JsonValue::as_str)
        .unwrap_or("global");
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let char_data = json!({
        "id": 0,
        "name": "[System]",
        "level": 99,
        "rank": 10,
        "account_type": 99,
        "character_sets": { "hair_style": 1 }
    })
    .to_string();
    let _ = sqlx::query(
        "INSERT INTO chat_messages (channel, character_id, character_name, character_data, message, timestamp)
         VALUES (?, 0, '[System]', ?, ?, ?)",
    )
    .bind(channel)
    .bind(char_data)
    .bind(&message)
    .bind(ts)
    .execute(pool)
    .await;

    json_resp(
        StatusCode::OK,
        json!({ "status": 1, "ok": true, "message": message, "channel": channel }),
    )
}
