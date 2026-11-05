//! HTTP REST routes for `/crew/*` and root-level crew aliases.
//!
//! Source:
//! - `routes/web.php`
//! - `app/Http/Middleware/CrewAuthMiddleware.php`
//! - `app/Http/Controllers/CrewApiController.php`
use crate::{
    GameState,
    clan_http::{now_ts, parse_body_params, verify_char_session_and_issue_token},
    rewards,
};
use authentication as auth;
use axum::{
    Json as AxumJson, Router,
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};
use std::collections::HashMap;

const CASTLE_NAMES: [&str; 7] = [
    "Hiroshima Castle",
    "Himeji Castle",
    "Kumamoto Castle",
    "Okazaki Castle",
    "Inuyama Castle",
    "Gifu Castle",
    "Hikone Castle",
];

fn signed_battle_request(char_id: i64, params: &HashMap<String, String>, phase_one: bool) -> bool {
    let value = |key: &str| params.get(key).map(String::as_str).unwrap_or("");
    let nonce = value("b");
    let timestamp = value("t").parse::<i64>().unwrap_or(0);
    let seconds = if timestamp > 10_000_000_000 {
        timestamp / 1000
    } else {
        timestamp
    };
    if !(8..=64).contains(&nonce.len()) || (now_ts() - seconds).abs() > 300 {
        return false;
    }
    let payload = if phase_one {
        format!(
            "{char_id}|{nonce}|{}|{}|{}",
            value("t"),
            value("c"),
            value("e")
        )
    } else {
        format!("{char_id}|{nonce}|{}|{}", value("t"), value("c"))
    };
    value("h").len() == 32 && auth::valid_legacy_hash(value("h"), &payload)
}

async fn start_phase_one(
    pool: &MySqlPool,
    char_id: i64,
    params: &HashMap<String, String>,
) -> std::result::Result<Json, sqlx::Error> {
    if !signed_battle_request(char_id, params, true) {
        return Ok(json!({"errorMessage":"Invalid battle request","code":422}));
    }
    let requested = params
        .get("c")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let enemy = params.get("e").map(String::as_str).unwrap_or("");
    let enemy_exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM enemies WHERE enemy_id=?")
        .bind(enemy)
        .fetch_one(pool)
        .await?;
    if enemy_exists == 0 {
        return Ok(json!({"errorMessage":"Enemy not found","code":422}));
    }
    if let Some(config) = crate::economy::catalog_entry("gamedata", "crew") {
        let allowed = config["data"]["boss"].as_array().is_some_and(|a| {
            a.iter().any(|boss| {
                boss["id"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(|v| v == enemy))
            })
        });
        if !allowed {
            return Ok(json!({"errorMessage":"Invalid crew boss","code":422}));
        }
    }
    let (season_id, _, phase) = ensure_crew_season(pool).await?;
    if phase != 1 {
        return Ok(json!({"errorMessage":"The crew season is no longer in phase one","code":409}));
    }
    ensure_castles_exist(pool, season_id).await?;
    let mut tx = pool.begin().await?;
    let Some(member) =
        sqlx::query("SELECT crew_id,stamina FROM crew_members WHERE character_id=? FOR UPDATE")
            .bind(char_id)
            .fetch_optional(&mut *tx)
            .await?
    else {
        return Ok(json!({"errorMessage":"Not in a crew","code":400}));
    };
    let crew_id: u64 = member.get("crew_id");
    let nonce = params.get("b").map(String::as_str).unwrap_or("");
    if let Some(saved)=sqlx::query_scalar::<_,Json>("SELECT battle_data FROM crew_battles WHERE season_id=? AND JSON_UNQUOTE(JSON_EXTRACT(battle_data,'$.character_id'))=? AND JSON_UNQUOTE(JSON_EXTRACT(battle_data,'$.request_nonce'))=? AND JSON_EXTRACT(battle_data,'$.phase')=1 ORDER BY id DESC LIMIT 1").bind(season_id).bind(char_id.to_string()).bind(nonce).fetch_optional(&mut *tx).await? {return Ok(saved["start"].clone());}
    if member.get::<i32, _>("stamina") < 10 {
        return Ok(json!({"errorMessage":"Not enough stamina","code":402}));
    }
    let castle = if (1..=7).contains(&requested) {
        sqlx::query("SELECT id,owner_crew_id FROM crew_castles WHERE season_id=? AND castle_index=? FOR UPDATE").bind(season_id).bind(requested-1).fetch_optional(&mut *tx).await?
    } else {
        sqlx::query(
            "SELECT id,owner_crew_id FROM crew_castles WHERE season_id=? AND id=? FOR UPDATE",
        )
        .bind(season_id)
        .bind(requested)
        .fetch_optional(&mut *tx)
        .await?
    };
    let Some(castle) = castle else {
        return Ok(json!({"errorMessage":"Castle not found","code":404}));
    };
    let castle_id: u64 = castle.get("id");
    let recruits: Vec<Json> = params
        .get("f")
        .and_then(|s| serde_json::from_str::<Json>(s).ok())
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    if recruits.len() > 2 {
        return Ok(json!({"errorMessage":"Maximum two recruits","code":422}));
    }
    let mut friends = vec![];
    for recruit in recruits {
        let target = recruit
            .as_u64()
            .or_else(|| {
                recruit
                    .as_str()
                    .and_then(|s| s.trim_start_matches("char_").parse().ok())
            })
            .unwrap_or(0);
        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM crew_members WHERE crew_id=? AND character_id=?",
        )
        .bind(crew_id)
        .bind(target)
        .fetch_one(&mut *tx)
        .await?;
        if exists == 0 || target == char_id as u64 || friends.contains(&target) {
            return Ok(json!({"errorMessage":"Invalid crew recruit","code":422}));
        }
        friends.push(target);
    }
    let code =
        auth::random_hex().map_err(|_| sqlx::Error::Protocol("Entropy unavailable".into()))?;
    let start = json!({"b":code,"c":requested,"t":now_ts(),"f":friends});
    let data = json!({"phase":1,"character_id":char_id,"request_nonce":nonce,"start":start,"completed":false,"enemy":enemy});
    sqlx::query("UPDATE crew_members SET stamina=stamina-10,updated_at=NOW() WHERE character_id=?")
        .bind(char_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO crew_battles(season_id,castle_id,attacker_crew_id,defender_crew_id,attacker_won,battle_data,created_at,updated_at) VALUES(?,?,?,?,0,?,NOW(),NOW())").bind(season_id).bind(castle_id).bind(crew_id).bind(castle.get::<Option<u64>,_>("owner_crew_id")).bind(data).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(start)
}

async fn finish_phase_one(
    pool: &MySqlPool,
    char_id: i64,
    params: &HashMap<String, String>,
) -> std::result::Result<Json, sqlx::Error> {
    let value = |key: &str| params.get(key).map(String::as_str).unwrap_or("");
    let code = value("b");
    let stats = value("s");
    if code.len() != 32 || stats.len() > 64000 {
        return Ok(json!({"errorMessage":"Invalid battle result","code":422}));
    }
    let payload = format!("{char_id}|{code}|{}|{}|{stats}", value("t"), value("c"));
    if value("h").len() != 32 || !auth::valid_legacy_hash(value("h"), &payload) {
        return Ok(json!({"errorMessage":"Invalid battle result","code":422}));
    }
    let mut tx = pool.begin().await?;
    let Some(member) =
        sqlx::query("SELECT crew_id,stamina FROM crew_members WHERE character_id=? FOR UPDATE")
            .bind(char_id)
            .fetch_optional(&mut *tx)
            .await?
    else {
        return Ok(json!({"errorMessage":"Not in a crew","code":400}));
    };
    let crew_id: u64 = member.get("crew_id");
    let Some(battle)=sqlx::query("SELECT id,castle_id,battle_data,UNIX_TIMESTAMP(created_at) AS created_ts FROM crew_battles WHERE attacker_crew_id=? AND JSON_UNQUOTE(JSON_EXTRACT(battle_data,'$.character_id'))=? AND JSON_UNQUOTE(JSON_EXTRACT(battle_data,'$.start.b'))=? AND JSON_EXTRACT(battle_data,'$.phase')=1 ORDER BY id DESC LIMIT 1 FOR UPDATE").bind(crew_id).bind(char_id.to_string()).bind(code).fetch_optional(&mut *tx).await? else{return Ok(json!({"errorMessage":"Battle session not found","code":404}));};
    let mut data: Json = battle.get("battle_data");
    if data["start"]["t"].as_i64() != value("t").parse::<i64>().ok()
        || data["start"]["c"].as_u64() != value("c").parse::<u64>().ok()
    {
        return Ok(json!({"errorMessage":"Battle session mismatch","code":422}));
    }
    if data["completed"] == true {
        return Ok(data["result"].clone());
    }
    if now_ts() - battle.get::<Option<i64>, _>("created_ts").unwrap_or(0) > 1800 {
        return Ok(json!({"errorMessage":"Battle session expired","code":410}));
    }
    let castle_id = battle.get::<Option<u64>, _>("castle_id").unwrap_or(0);
    let Some(castle) =
        sqlx::query("SELECT owner_crew_id,wall_hp FROM crew_castles WHERE id=? FOR UPDATE")
            .bind(castle_id)
            .fetch_optional(&mut *tx)
            .await?
    else {
        return Ok(json!({"errorMessage":"Castle not found","code":404}));
    };
    let owner: Option<u64> = castle.get("owner_crew_id");
    let damage = if owner == Some(crew_id) {
        0
    } else {
        rand_range(5, 20)
    };
    let wall = i64::from(castle.get::<u32, _>("wall_hp"));
    let captured = owner != Some(crew_id) && wall <= damage;
    let merit = if captured {
        rand_range(50, 150)
    } else {
        rand_range(10, 50)
    };
    if captured {
        sqlx::query("UPDATE crew_castles SET owner_crew_id=?,wall_hp=100,defender_hp=100,updated_at=NOW() WHERE id=?").bind(crew_id).bind(castle_id).execute(&mut *tx).await?;
        sqlx::query("UPDATE crews SET prestige=prestige+10,updated_at=NOW() WHERE id=?")
            .bind(crew_id)
            .execute(&mut *tx)
            .await?;
    } else if damage > 0 {
        sqlx::query("UPDATE crew_castles SET wall_hp=?,updated_at=NOW() WHERE id=?")
            .bind((wall - damage).max(0))
            .bind(castle_id)
            .execute(&mut *tx)
            .await?;
    }
    let result = json!({"success":true,"data":{"m":merit,"d":damage},"stamina":member.get::<i32,_>("stamina"),"captured":captured});
    data["completed"] = json!(true);
    data["merit"] = json!(merit);
    data["damage"] = json!(damage);
    data["result"] = result.clone();
    sqlx::query("UPDATE crew_battles SET attacker_won=1,battle_data=?,updated_at=NOW() WHERE id=?")
        .bind(data)
        .bind(battle.get::<u64, _>("id"))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(result)
}

async fn phase_two_result(
    pool: &MySqlPool,
    char_id: i64,
    params: &HashMap<String, String>,
) -> std::result::Result<Json, sqlx::Error> {
    let Some(nonce) = params.get("b") else {
        return Ok(json!({"errorMessage":"Battle nonce required","code":422}));
    };
    let saved:Option<Json>=sqlx::query_scalar("SELECT battle_data FROM crew_battles WHERE JSON_UNQUOTE(JSON_EXTRACT(battle_data,'$.character_id'))=? AND JSON_UNQUOTE(JSON_EXTRACT(battle_data,'$.request_nonce'))=? AND JSON_EXTRACT(battle_data,'$.phase')=2 ORDER BY id DESC LIMIT 1").bind(char_id.to_string()).bind(nonce).fetch_optional(pool).await?;
    Ok(saved
        .map(|v| v["result"].clone())
        .unwrap_or_else(|| json!({"errorMessage":"Battle result not found","code":404})))
}

pub fn router() -> Router<GameState> {
    Router::new()
        .route("/crew/season", post(crew_dispatch_root))
        .route("/crew/timestamps", post(crew_dispatch_root))
        .route("/crew/histories", post(crew_dispatch_root))
        .route("/crew/announcements", post(crew_dispatch_root))
        .route("/crew/create", post(crew_dispatch_root))
        .route("/crew/rename", post(crew_dispatch_root))
        .route("/crew/season-histories", post(crew_dispatch_root))
        .route("/crew/{*rest}", post(crew_dispatch_rest))
        // Root-level crew aliases from routes/web.php
        .route("/auth/login", post(crew_dispatch_alias))
        .route("/season", post(crew_dispatch_alias))
        .route("/season/{*rest}", post(crew_dispatch_alias))
        .route("/timestamps", post(crew_dispatch_alias))
        .route("/histories", post(crew_dispatch_alias))
        .route("/history", post(crew_dispatch_alias))
        .route("/announcements", post(crew_dispatch_alias))
        .route("/create", post(crew_dispatch_alias))
        .route("/rename", post(crew_dispatch_alias))
        .route("/season-histories", post(crew_dispatch_alias))
        .route("/player/{*rest}", post(crew_dispatch_alias))
        .route("/request/{*rest}", post(crew_dispatch_alias))
        .route("/member/{*rest}", post(crew_dispatch_alias))
        .route("/upgrade/{*rest}", post(crew_dispatch_alias))
        .route("/announcement/{*rest}", post(crew_dispatch_alias))
        .route("/battle/{*rest}", post(crew_dispatch_alias))
}

async fn crew_dispatch_root(
    State(state): State<GameState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
    uri: axum::http::Uri,
    bytes: Bytes,
) -> Response {
    let rest = uri.path().strip_prefix("/crew/").unwrap_or("");
    handle_crew(state, headers, query, rest.to_string(), bytes).await
}

async fn crew_dispatch_rest(
    State(state): State<GameState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
    Path(rest): Path<String>,
    bytes: Bytes,
) -> Response {
    handle_crew(state, headers, query, rest, bytes).await
}

async fn crew_dispatch_alias(
    State(state): State<GameState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
    uri: axum::http::Uri,
    bytes: Bytes,
) -> Response {
    let rest = uri.path().trim_start_matches('/');
    handle_crew(state, headers, query, rest.to_string(), bytes).await
}

async fn handle_crew(
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
        return match ensure_crew_season(pool).await {
            Ok((sid, snum, phase)) => AxumJson(json!({
                "id": sid,
                "number": snum,
                "timestamp": 86400,
                "season": {
                    "id": sid,
                    "number": snum,
                    "season": snum,
                    "active": true,
                    "phase": phase,
                    "timestamp": 86400,
                },
                "phase": phase,
                "rp1": ["back_8001"],
                "rp2": [
                    "back_8002", "wpn_8001", "skill_8001", "wpn_8002",
                    "accessory_8001", "back_8003", "wpn_8003"
                ],
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
        return verify_char_session_and_issue_token(pool, "crew_auth_tokens", &params).await;
    }
    let token = params.get("token").map(String::as_str).unwrap_or("");
    let auth_res = match authenticate_crew_token(pool, token).await {
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
    match dispatch_crew_route(pool, user_id, char_id, clean_rest, &params).await {
        Ok(val) => AxumJson(val).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            AxumJson(json!({"errorMessage": "Database operation failed"})),
        )
            .into_response(),
    }
}

async fn authenticate_crew_token(
    pool: &MySqlPool,
    token: &str,
) -> std::result::Result<Option<(i64, i64)>, sqlx::Error> {
    if token.trim().is_empty() {
        return Ok(None);
    }
    let row = sqlx::query(
        "SELECT user_id, character_id FROM crew_auth_tokens WHERE token=? AND expires_at > NOW() LIMIT 1",
    )
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

async fn ensure_crew_season(pool: &MySqlPool) -> std::result::Result<(u64, u32, u8), sqlx::Error> {
    if let Some(row) =
        sqlx::query("SELECT id, number, phase FROM crew_seasons WHERE active=1 LIMIT 1")
            .fetch_optional(pool)
            .await?
    {
        return Ok((row.get("id"), row.get("number"), row.get("phase")));
    }
    let res = sqlx::query(
        "INSERT INTO crew_seasons (number, active, phase, started_at, created_at, updated_at) \
         VALUES (1, 1, 1, NOW(), NOW(), NOW())",
    )
    .execute(pool)
    .await?;
    Ok((res.last_insert_id(), 1, 1))
}

async fn ensure_castles_exist(
    pool: &MySqlPool,
    season_id: u64,
) -> std::result::Result<Vec<sqlx::mysql::MySqlRow>, sqlx::Error> {
    let cnt: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM crew_castles WHERE season_id=?")
        .bind(season_id)
        .fetch_one(pool)
        .await?;
    if cnt < 7 {
        for (i, &name) in CASTLE_NAMES.iter().enumerate() {
            sqlx::query(
                "INSERT IGNORE INTO crew_castles (season_id, castle_index, name, wall_hp, defender_hp, created_at, updated_at) \
                 VALUES (?, ?, ?, 100, 100, NOW(), NOW())",
            )
            .bind(season_id)
            .bind(i as u8)
            .bind(name)
            .execute(pool)
            .await?;
        }
    }
    sqlx::query("SELECT * FROM crew_castles WHERE season_id=? ORDER BY castle_index ASC")
        .bind(season_id)
        .fetch_all(pool)
        .await
}

async fn format_crew_full(
    pool: &MySqlPool,
    crew_id: u64,
) -> std::result::Result<Option<Json>, sqlx::Error> {
    let Some(c_row) = sqlx::query("SELECT * FROM crews WHERE id=?")
        .bind(crew_id)
        .fetch_optional(pool)
        .await?
    else {
        return Ok(None);
    };
    let master_id = c_row.get::<u64, _>("master_id") as i64;
    let master_name: String = sqlx::query_scalar("SELECT name FROM characters WHERE id=?")
        .bind(master_id)
        .fetch_optional(pool)
        .await?
        .unwrap_or_default();
    let elder_row = sqlx::query(
        "SELECT cm.character_id, c.name FROM crew_members cm JOIN characters c ON c.id = cm.character_id \
         WHERE cm.crew_id=? AND cm.role='elder' LIMIT 1",
    )
    .bind(crew_id)
    .fetch_optional(pool)
    .await?;
    let (elder_id, elder_name) = match elder_row {
        Some(r) => (r.get::<u64, _>("character_id"), r.get::<String, _>("name")),
        None => (0, String::new()),
    };
    let members_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM crew_members WHERE crew_id=?")
            .bind(crew_id)
            .fetch_one(pool)
            .await?;
    let b_json = c_row
        .try_get::<Option<Json>, _>("buildings")
        .ok()
        .flatten()
        .unwrap_or_else(|| json!({}));

    Ok(Some(json!({
        "id": crew_id,
        "name": c_row.get::<String, _>("name"),
        "prestige": c_row.get::<i32, _>("prestige"),
        "master_id": master_id,
        "master_name": master_name,
        "elder_id": elder_id,
        "elder_name": elder_name,
        "max_members": c_row.get::<i32, _>("max_members"),
        "members": members_count,
        "golds": c_row.get::<i32, _>("gold"),
        "tokens": c_row.get::<i32, _>("tokens"),
        "announcement": c_row.get::<Option<String>, _>("announcement_published").unwrap_or_default(),
        "announcement_draft": c_row.get::<Option<String>, _>("announcement_draft").unwrap_or_default(),
        "kushi_dango": b_json.get("kushi_dango").and_then(Json::as_i64).unwrap_or(0),
        "tea_house": b_json.get("tea_house").and_then(Json::as_i64).unwrap_or(0),
        "bath_house": b_json.get("bath_house").and_then(Json::as_i64).unwrap_or(0),
        "training_centre": b_json.get("training_centre").and_then(Json::as_i64).unwrap_or(0),
    })))
}

async fn crew_character(pool: &MySqlPool, char_id: i64) -> std::result::Result<Json, sqlx::Error> {
    let Some(r)=sqlx::query("SELECT m.*,c.name,c.level,UNIX_TIMESTAMP(m.prestige_boost_expires_at) AS boost_ts FROM crew_members m JOIN characters c ON c.id=m.character_id WHERE m.character_id=?").bind(char_id).fetch_optional(pool).await? else {return Ok(Json::Null);};
    Ok(
        json!({"char_id":char_id,"name":r.get::<String,_>("name"),"level":r.get::<i32,_>("level"),"role":r.get::<u8,_>("battle_role"),"member_role":r.get::<String,_>("role"),"battle_role":r.get::<u8,_>("battle_role"),"role_limit_at":r.get::<String,_>("role_limit_at"),"stamina":r.get::<i32,_>("stamina"),"max_stamina":r.get::<i32,_>("max_stamina"),"prestige":0,"prestige_boost":(r.try_get::<Option<i64>,_>("boost_ts")?.unwrap_or(0)-now_ts()).max(0)}),
    )
}

async fn format_crew_members(
    pool: &MySqlPool,
    crew_id: u64,
    battle_role_filter: Option<u8>,
) -> std::result::Result<Vec<Json>, sqlx::Error> {
    let rows = if let Some(br) = battle_role_filter {
        sqlx::query(
            "SELECT cm.*, UNIX_TIMESTAMP(cm.prestige_boost_expires_at) AS boost_ts, c.name, c.level, (SELECT CAST(COALESCE(SUM(CAST(JSON_UNQUOTE(JSON_EXTRACT(cb.battle_data,'$.damage')) AS SIGNED)),0) AS SIGNED) FROM crew_battles cb WHERE cb.attacker_crew_id=cm.crew_id AND JSON_UNQUOTE(JSON_EXTRACT(cb.battle_data,'$.character_id'))=CAST(cm.character_id AS CHAR)) AS damage_total, (SELECT COUNT(*) FROM crew_battles cb WHERE cb.attacker_crew_id=cm.crew_id AND cb.attacker_won=1 AND JSON_UNQUOTE(JSON_EXTRACT(cb.battle_data,'$.character_id'))=CAST(cm.character_id AS CHAR)) AS boss_kills \
             FROM crew_members cm LEFT JOIN characters c ON c.id = cm.character_id \
             WHERE cm.crew_id=? AND cm.battle_role=?",
        )
        .bind(crew_id)
        .bind(br)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT cm.*, UNIX_TIMESTAMP(cm.prestige_boost_expires_at) AS boost_ts, c.name, c.level, (SELECT CAST(COALESCE(SUM(CAST(JSON_UNQUOTE(JSON_EXTRACT(cb.battle_data,'$.damage')) AS SIGNED)),0) AS SIGNED) FROM crew_battles cb WHERE cb.attacker_crew_id=cm.crew_id AND JSON_UNQUOTE(JSON_EXTRACT(cb.battle_data,'$.character_id'))=CAST(cm.character_id AS CHAR)) AS damage_total, (SELECT COUNT(*) FROM crew_battles cb WHERE cb.attacker_crew_id=cm.crew_id AND cb.attacker_won=1 AND JSON_UNQUOTE(JSON_EXTRACT(cb.battle_data,'$.character_id'))=CAST(cm.character_id AS CHAR)) AS boss_kills \
             FROM crew_members cm LEFT JOIN characters c ON c.id = cm.character_id \
             WHERE cm.crew_id=?",
        )
        .bind(crew_id)
        .fetch_all(pool)
        .await?
    };
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
                "stamina": r.get::<i32, _>("stamina"),
                "max_stamina": r.get::<i32, _>("max_stamina"),
                "role": r.get::<String, _>("role"),
                "battle_role": r.get::<u8, _>("battle_role"),
                "role_limit_at": r.get::<String, _>("role_limit_at"),
                "donated_golds": r.get::<i32, _>("donated_golds"),
                "gold_donated":r.get::<i32,_>("donated_golds"),
                "token_donated":r.get::<i32,_>("donated_tokens"),
                "damage":r.get::<i64,_>("damage_total"),"boss_kill":r.get::<i64,_>("boss_kills"),
                "donated_tokens": r.get::<i32, _>("donated_tokens"),
                "prestige_boost": (boost_ts - now).max(0),
            })
        })
        .collect())
}

async fn format_crew_summaries(
    pool: &MySqlPool,
    where_clause: &str,
    bind_val: Option<String>,
    limit: i64,
) -> std::result::Result<Vec<Json>, sqlx::Error> {
    let sql = format!(
        "SELECT c.id, c.name, c.prestige, c.max_members, \
         (SELECT COUNT(*) FROM crew_members cm WHERE cm.crew_id = c.id) AS m_cnt \
         FROM crews c {where_clause} ORDER BY c.prestige DESC LIMIT {limit}"
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
                "prestige": r.get::<i32, _>("prestige"),
                "max_members": r.get::<i32, _>("max_members"),
                "members": r.get::<i64, _>("m_cnt"),
            })
        })
        .collect())
}

fn rand_range(min: i64, max: i64) -> i64 {
    let hex = auth::random_hex().unwrap_or_else(|_| "0123456789abcdef".into());
    let val = u64::from_str_radix(&hex[..8], 16).unwrap_or(0);
    min + (val as i64 % (max - min + 1).max(1))
}

async fn dispatch_crew_route(
    pool: &MySqlPool,
    user_id: i64,
    char_id: i64,
    path: &str,
    params: &HashMap<String, String>,
) -> std::result::Result<Json, sqlx::Error> {
    let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    match segs.as_slice() {
        ["season"] => {
            let (sid, snum, phase) = ensure_crew_season(pool).await?;
            Ok(json!({
                "season": {
                    "id": sid,
                    "season": snum,
                    "active": true,
                    "phase": phase,
                    "timestamp": 86400,
                }
            }))
        }
        ["season", "pool"] => Ok(json!({ "t": 0 })),
        ["season", "previous"] => Ok(json!({ "rewards": [], "season": null })),
        ["timestamps"] => {
            let (_, snum, phase) = ensure_crew_season(pool).await?;
            Ok(json!({
                "s": snum,
                "p": phase,
                "e": format!("Phase {phase} Ends In"),
                "c": 86400,
            }))
        }
        ["player", "data"] => {
            let member = sqlx::query(
                "SELECT cm.*, UNIX_TIMESTAMP(cm.prestige_boost_expires_at) AS boost_ts \
                 FROM crew_members cm WHERE cm.character_id=?",
            )
            .bind(char_id)
            .fetch_optional(pool)
            .await?;
            let Some(m) = member else {
                return Ok(json!({ "crew_data": null }));
            };
            let crew_id: u64 = m.get("crew_id");
            let crew_data = format_crew_full(pool, crew_id).await?;
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
                "crew_data": crew_data,
                "char_data": {
                    "char_id": char_id,
                    "name": c_name,
                    "level": c_lvl,
                    "stamina": m.get::<i32, _>("stamina"),
                    "max_stamina": m.get::<i32, _>("max_stamina"),
                    "role": m.get::<String, _>("role"),
                    "battle_role": m.get::<u8, _>("battle_role"),
                    "role_limit_at": m.get::<String, _>("role_limit_at"),
                    "donated_golds": m.get::<i32, _>("donated_golds"),
                    "donated_tokens": m.get::<i32, _>("donated_tokens"),
                    "prestige_boost": (boost_ts - now_ts()).max(0),
                    "onigiri_data": onigiri_qty,
                }
            }))
        }
        ["player", "crew", "members"] => {
            let crew_id: Option<u64> =
                sqlx::query_scalar("SELECT crew_id FROM crew_members WHERE character_id=?")
                    .bind(char_id)
                    .fetch_optional(pool)
                    .await?;
            let members = if let Some(id) = crew_id {
                format_crew_members(pool, id, None).await?
            } else {
                vec![]
            };
            Ok(json!({"members":members}))
        }
        ["player", "stamina"] => {
            let (sid, num, phase) = ensure_crew_season(pool).await?;
            Ok(
                json!({"char":crew_character(pool,char_id).await?,"season":{"id":sid,"number":num,"phase":phase,"timestamp":86400}}),
            )
        }
        ["player", "crew"] => {
            let member = sqlx::query("SELECT crew_id FROM crew_members WHERE character_id=?")
                .bind(char_id)
                .fetch_optional(pool)
                .await?;
            let Some(m) = member else {
                let crews = format_crew_summaries(pool, "", None, 50).await?;
                return Ok(json!({ "code":404,"crew": null, "crews": crews }));
            };
            let crew_id: u64 = m.get("crew_id");
            let crew = format_crew_full(pool, crew_id).await?;
            let members = format_crew_members(pool, crew_id, None).await?;
            Ok(json!({
                "crew": crew,
                "char":crew_character(pool,char_id).await?,
                "members": members,
                "date": "",
            }))
        }
        ["histories"] | ["history"] => {
            let Some(crew_id) = sqlx::query_scalar::<_, u64>(
                "SELECT crew_id FROM crew_members WHERE character_id=?",
            )
            .bind(char_id)
            .fetch_optional(pool)
            .await?
            else {
                return Ok(json!({ "histories": [] }));
            };
            let rows = sqlx::query(
                "SELECT cb.*, a.name AS attacker_name, d.name AS defender_name \
                 FROM crew_battles cb \
                 LEFT JOIN crews a ON a.id = cb.attacker_crew_id \
                 LEFT JOIN crews d ON d.id = cb.defender_crew_id \
                 WHERE cb.attacker_crew_id=? OR cb.defender_crew_id=? \
                 ORDER BY cb.created_at DESC, cb.id DESC LIMIT 20",
            )
            .bind(crew_id)
            .bind(crew_id)
            .fetch_all(pool)
            .await?;
            let histories: Vec<Json> = rows
                .into_iter()
                .map(|r| {
                    let att_id: u64 = r.get("attacker_crew_id");
                    let def_id = r.get::<Option<u64>, _>("defender_crew_id").unwrap_or(0);
                    let is_att = att_id == crew_id;
                    let att_won: bool = r.get("attacker_won");
                    let won = if is_att { att_won } else { !att_won };
                    let opp_id = if is_att { def_id } else { att_id };
                    let opp_name = if is_att {
                        r.get::<Option<String>, _>("defender_name")
                    } else {
                        r.get::<Option<String>, _>("attacker_name")
                    }
                    .unwrap_or_else(|| "Neutral".into());
                    let rep_str = if is_att {
                        if won { "+10" } else { "0" }
                    } else if won {
                        "0"
                    } else {
                        "-5"
                    };
                    json!({
                        "detail": format!("Crew {} ({})", opp_name, opp_id),
                        "True": if won { 1 } else { 0 },
                        "reputation": rep_str,
                        "date": "just now",
                    })
                })
                .collect();
            Ok(json!({ "histories": histories }))
        }
        ["request", "filtered"] => {
            let name = params.get("name").cloned().unwrap_or_default();
            let id = params.get("id").cloned().unwrap_or_default();
            let crews = if !id.is_empty() {
                format_crew_summaries(pool, "WHERE c.id = ?", Some(id), 20).await?
            } else if !name.is_empty() {
                format_crew_summaries(pool, "WHERE c.name LIKE ?", Some(format!("%{name}%")), 20)
                    .await?
            } else {
                format_crew_summaries(pool, "", None, 20).await?
            };
            Ok(json!({ "crews": crews }))
        }
        ["request", "available"] => {
            let crews = format_crew_summaries(pool, "", None, 50).await?;
            Ok(json!({ "crews": crews }))
        }
        ["request", "send", crew_id_str] | ["player", "request", crew_id_str] => {
            let target_crew_id = crew_id_str.parse::<u64>().unwrap_or(0);
            let in_crew: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM crew_members WHERE character_id=?")
                    .bind(char_id)
                    .fetch_one(pool)
                    .await?;
            if in_crew > 0 {
                return Ok(json!({ "errorMessage": "Already in a crew" }));
            }
            let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM crews WHERE id=?")
                .bind(target_crew_id)
                .fetch_one(pool)
                .await?;
            if exists == 0 {
                return Ok(json!({ "errorMessage": "Crew not found" }));
            }
            sqlx::query(
                "INSERT INTO crew_requests (crew_id, character_id, status, created_at, updated_at) \
                 VALUES (?, ?, 'pending', NOW(), NOW()) \
                 ON DUPLICATE KEY UPDATE status='pending', updated_at=NOW()",
            )
            .bind(target_crew_id)
            .bind(char_id)
            .execute(pool)
            .await?;
            Ok(json!({ "result": "Request to join crew sent!" }))
        }
        ["request", "all"] => {
            let Some(m) =
                sqlx::query("SELECT crew_id, role FROM crew_members WHERE character_id=?")
                    .bind(char_id)
                    .fetch_optional(pool)
                    .await?
            else {
                return Ok(json!({ "requests": [] }));
            };
            let role: String = m.get("role");
            if role != "master" && role != "elder" {
                return Ok(json!({ "requests": [] }));
            }
            let crew_id: u64 = m.get("crew_id");
            let rows = sqlx::query(
                "SELECT cr.id, cr.character_id, c.name, c.level \
                 FROM crew_requests cr LEFT JOIN characters c ON c.id = cr.character_id \
                 WHERE cr.crew_id=? AND cr.status='pending'",
            )
            .bind(crew_id)
            .fetch_all(pool)
            .await?;
            let members: Vec<Json> = rows
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
            Ok(json!({ "total":members.len(),"requests": members }))
        }
        ["request", "reject", req_char_str] | ["request", req_char_str, "reject"]
            if *req_char_str != "all" =>
        {
            let req_char_id = req_char_str.parse::<u64>().unwrap_or(0);
            let Some(m) =
                sqlx::query("SELECT crew_id, role FROM crew_members WHERE character_id=?")
                    .bind(char_id)
                    .fetch_optional(pool)
                    .await?
            else {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            };
            let role: String = m.get("role");
            if role != "master" && role != "elder" {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            }
            let crew_id: u64 = m.get("crew_id");
            sqlx::query(
                "UPDATE crew_requests SET status='rejected', updated_at=NOW() \
                 WHERE crew_id=? AND character_id=? AND status='pending'",
            )
            .bind(crew_id)
            .bind(req_char_id)
            .execute(pool)
            .await?;
            Ok(json!({ "success": true }))
        }
        ["request", "reject-all"] | ["request", "all", "reject"] => {
            let Some(m) =
                sqlx::query("SELECT crew_id, role FROM crew_members WHERE character_id=?")
                    .bind(char_id)
                    .fetch_optional(pool)
                    .await?
            else {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            };
            let role: String = m.get("role");
            if role != "master" && role != "elder" {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            }
            sqlx::query(
                "UPDATE crew_requests SET status='rejected', updated_at=NOW() WHERE crew_id=? AND status='pending'",
            )
            .bind(m.get::<u64, _>("crew_id"))
            .execute(pool)
            .await?;
            Ok(json!({ "success": true }))
        }
        ["request", "accept", req_char_str] | ["request", req_char_str, "accept"] => {
            let req_char_id = req_char_str.parse::<u64>().unwrap_or(0);
            let Some(m) =
                sqlx::query("SELECT crew_id, role FROM crew_members WHERE character_id=?")
                    .bind(char_id)
                    .fetch_optional(pool)
                    .await?
            else {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            };
            let role: String = m.get("role");
            if role != "master" && role != "elder" {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            }
            let crew_id: u64 = m.get("crew_id");
            let Some(req_row) = sqlx::query(
                "SELECT id, character_id FROM crew_requests WHERE crew_id=? AND character_id=? AND status='pending' LIMIT 1",
            )
            .bind(crew_id)
            .bind(req_char_id)
            .fetch_optional(pool)
            .await?
            else {
                return Ok(json!({ "errorMessage": "Request not found" }));
            };
            let req_id: u64 = req_row.get("id");
            let target_char_id: u64 = req_row.get("character_id");
            let max_members: i32 = sqlx::query_scalar("SELECT max_members FROM crews WHERE id=?")
                .bind(crew_id)
                .fetch_one(pool)
                .await?;
            let cur_cnt: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM crew_members WHERE crew_id=?")
                    .bind(crew_id)
                    .fetch_one(pool)
                    .await?;
            if cur_cnt >= i64::from(max_members) {
                return Ok(json!({ "errorMessage": "Crew is full" }));
            }
            let mut tx = pool.begin().await?;
            sqlx::query("DELETE FROM crew_members WHERE character_id=?")
                .bind(target_char_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "INSERT INTO crew_members (crew_id, character_id, role, battle_role, stamina, max_stamina, created_at, updated_at) \
                 VALUES (?, ?, 'member', 1, 50, 50, NOW(), NOW())",
            )
            .bind(crew_id)
            .bind(target_char_id)
            .execute(&mut *tx)
            .await?;
            sqlx::query("UPDATE crew_requests SET status='accepted', updated_at=NOW() WHERE id=?")
                .bind(req_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "UPDATE crew_requests SET status='rejected', updated_at=NOW() WHERE character_id=? AND id!=? AND status='pending'",
            )
            .bind(target_char_id)
            .bind(req_id)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            Ok(json!({ "success": true }))
        }
        ["member", "invite", target_str] | ["request", target_str, "invite"] => {
            let target_id = target_str.parse::<i64>().unwrap_or(0);
            let Some(actor) =
                sqlx::query("SELECT crew_id, role FROM crew_members WHERE character_id=?")
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
            let in_crew: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM crew_members WHERE character_id=?")
                    .bind(target_id)
                    .fetch_one(pool)
                    .await?;
            if in_crew > 0 {
                return Ok(json!({ "errorMessage": "Player is already in a crew" }));
            }
            let crew_id: u64 = actor.get("crew_id");
            let c_row = sqlx::query("SELECT name, max_members FROM crews WHERE id=?")
                .bind(crew_id)
                .fetch_one(pool)
                .await?;
            let cur_cnt: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM crew_members WHERE crew_id=?")
                    .bind(crew_id)
                    .fetch_one(pool)
                    .await?;
            if cur_cnt >= i64::from(c_row.get::<i32, _>("max_members")) {
                return Ok(json!({ "errorMessage": "Crew is full" }));
            }
            let c_name: String = c_row.get("name");
            sqlx::query(
                "INSERT INTO character_mails (character_id, type, title, sender, body, claimed, viewed, created_at, updated_at) \
                 VALUES (?, 'crew_invite', ?, ?, ?, 0, 0, NOW(), NOW())",
            )
            .bind(target_id)
            .bind(format!("{c_name} invites you to join their crew!"))
            .bind(&c_name)
            .bind(serde_json::to_string(&json!({ "crew_id": crew_id })).unwrap_or_default())
            .execute(pool)
            .await?;
            Ok(json!({ "success": true }))
        }
        ["player", "quit"] => {
            let Some(m) = sqlx::query("SELECT role FROM crew_members WHERE character_id=?")
                .bind(char_id)
                .fetch_optional(pool)
                .await?
            else {
                return Ok(json!({ "errorMessage": "Not in a crew" }));
            };
            if m.get::<String, _>("role") == "master" {
                return Ok(
                    json!({ "errorMessage": "Master cannot quit. Transfer leadership first." }),
                );
            }
            sqlx::query("DELETE FROM crew_members WHERE character_id=?")
                .bind(char_id)
                .execute(pool)
                .await?;
            Ok(json!({ "success": true }))
        }
        ["player", "kick", target_str] => {
            let target_id = target_str.parse::<i64>().unwrap_or(0);
            let Some(actor) =
                sqlx::query("SELECT crew_id, role FROM crew_members WHERE character_id=?")
                    .bind(char_id)
                    .fetch_optional(pool)
                    .await?
            else {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            };
            let a_role: String = actor.get("role");
            if a_role != "master" && a_role != "elder" {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            }
            let crew_id: u64 = actor.get("crew_id");
            let Some(target) =
                sqlx::query("SELECT role FROM crew_members WHERE crew_id=? AND character_id=?")
                    .bind(crew_id)
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
            sqlx::query("DELETE FROM crew_members WHERE crew_id=? AND character_id=?")
                .bind(crew_id)
                .bind(target_id)
                .execute(pool)
                .await?;
            Ok(json!({ "success": true }))
        }
        ["player", "promote-elder", target_str] => {
            let target_id = target_str.parse::<i64>().unwrap_or(0);
            let Some(actor) =
                sqlx::query("SELECT crew_id, role FROM crew_members WHERE character_id=?")
                    .bind(char_id)
                    .fetch_optional(pool)
                    .await?
            else {
                return Ok(json!({ "errorMessage": "Only master can promote" }));
            };
            if actor.get::<String, _>("role") != "master" {
                return Ok(json!({ "errorMessage": "Only master can promote" }));
            }
            let crew_id: u64 = actor.get("crew_id");
            let Some(target) =
                sqlx::query("SELECT role FROM crew_members WHERE crew_id=? AND character_id=?")
                    .bind(crew_id)
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
            sqlx::query(
                "UPDATE crew_members SET role=?, updated_at=NOW() WHERE crew_id=? AND character_id=?",
            )
            .bind(new_role)
            .bind(crew_id)
            .bind(target_id)
            .execute(pool)
            .await?;
            Ok(json!({ "success": true, "role": new_role }))
        }
        ["player", "switch-master", target_str] => {
            let target_id = target_str.parse::<i64>().unwrap_or(0);
            let Some(actor) =
                sqlx::query("SELECT crew_id, role FROM crew_members WHERE character_id=?")
                    .bind(char_id)
                    .fetch_optional(pool)
                    .await?
            else {
                return Ok(json!({ "errorMessage": "Only master can transfer leadership" }));
            };
            if actor.get::<String, _>("role") != "master" {
                return Ok(json!({ "errorMessage": "Only master can transfer leadership" }));
            }
            let crew_id: u64 = actor.get("crew_id");
            let exists: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM crew_members WHERE crew_id=? AND character_id=?",
            )
            .bind(crew_id)
            .bind(target_id)
            .fetch_one(pool)
            .await?;
            if exists == 0 {
                return Ok(json!({ "errorMessage": "Member not found" }));
            }
            let mut tx = pool.begin().await?;
            sqlx::query(
                "UPDATE crew_members SET role='member', updated_at=NOW() WHERE crew_id=? AND character_id=?",
            )
            .bind(crew_id)
            .bind(char_id)
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                "UPDATE crew_members SET role='master', updated_at=NOW() WHERE crew_id=? AND character_id=?",
            )
            .bind(crew_id)
            .bind(target_id)
            .execute(&mut *tx)
            .await?;
            sqlx::query("UPDATE crews SET master_id=?, updated_at=NOW() WHERE id=?")
                .bind(target_id)
                .bind(crew_id)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            Ok(json!({ "success": true }))
        }
        ["player", "donate", amount_str, "golds"] => {
            let amount = amount_str.parse::<i64>().unwrap_or(0);
            if amount <= 0 {
                return Ok(json!({ "errorMessage": "Invalid amount" }));
            }
            let mut tx = pool.begin().await?;
            let Some(m) =
                sqlx::query("SELECT crew_id FROM crew_members WHERE character_id=? FOR UPDATE")
                    .bind(char_id)
                    .fetch_optional(&mut *tx)
                    .await?
            else {
                return Ok(json!({ "errorMessage": "Not in a crew" }));
            };
            let crew_id: u64 = m.get("crew_id");
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
            sqlx::query(
                "UPDATE crew_members SET donated_golds = donated_golds + ?, updated_at=NOW() WHERE character_id=?",
            )
            .bind(amount)
            .bind(char_id)
            .execute(&mut *tx)
            .await?;
            sqlx::query("UPDATE crews SET gold = gold + ?, updated_at=NOW() WHERE id=?")
                .bind(amount)
                .bind(crew_id)
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
                sqlx::query("SELECT crew_id FROM crew_members WHERE character_id=? FOR UPDATE")
                    .bind(char_id)
                    .fetch_optional(&mut *tx)
                    .await?
            else {
                return Ok(json!({ "errorMessage": "Not in a crew" }));
            };
            let crew_id: u64 = m.get("crew_id");
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
            sqlx::query(
                "UPDATE crew_members SET donated_tokens = donated_tokens + ?, updated_at=NOW() WHERE character_id=?",
            )
            .bind(amount)
            .bind(char_id)
            .execute(&mut *tx)
            .await?;
            sqlx::query("UPDATE crews SET tokens = tokens + ?, updated_at=NOW() WHERE id=?")
                .bind(amount)
                .bind(crew_id)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            Ok(json!({ "success": true, "tokens": new_tokens }))
        }
        ["upgrade", "building", building_id] => {
            let b_key = match *building_id {
                "crew_kushi_dango" | "kushi_dango" => "kushi_dango",
                "crew_tea_house" | "tea_house" => "tea_house",
                "crew_bath_house" | "bath_house" => "bath_house",
                "crew_training_centre" | "training_centre" => "training_centre",
                _ => return Ok(json!({ "errorMessage": "Invalid building" })),
            };
            let mut tx = pool.begin().await?;
            let Some(m) = sqlx::query(
                "SELECT crew_id, role FROM crew_members WHERE character_id=? FOR UPDATE",
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
            let crew_id: u64 = m.get("crew_id");
            let c_row =
                sqlx::query("SELECT gold, tokens, buildings FROM crews WHERE id=? FOR UPDATE")
                    .bind(crew_id)
                    .fetch_one(&mut *tx)
                    .await?;
            let mut c_gold = i64::from(c_row.get::<i32, _>("gold"));
            let mut c_tokens = i64::from(c_row.get::<i32, _>("tokens"));
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
                    return Ok(json!({ "errorMessage": "Not enough crew gold" }));
                }
                c_gold -= cost;
                buildings.insert(b_key.into(), json!(cur_lvl + 1));
                sqlx::query("UPDATE crews SET gold=?, buildings=?, updated_at=NOW() WHERE id=?")
                    .bind(c_gold)
                    .bind(serde_json::to_string(&buildings).unwrap_or_else(|_| "{}".into()))
                    .bind(crew_id)
                    .execute(&mut *tx)
                    .await?;
            } else {
                let cost = 4000;
                if c_tokens < cost {
                    return Ok(json!({ "errorMessage": "Not enough crew tokens" }));
                }
                c_tokens -= cost;
                buildings.insert(b_key.into(), json!(cur_lvl + 1));
                sqlx::query("UPDATE crews SET tokens=?, buildings=?, updated_at=NOW() WHERE id=?")
                    .bind(c_tokens)
                    .bind(serde_json::to_string(&buildings).unwrap_or_else(|_| "{}".into()))
                    .bind(crew_id)
                    .execute(&mut *tx)
                    .await?;
            }
            tx.commit().await?;
            Ok(json!({
                "success": true,
                "kushi_dango": buildings.get("kushi_dango").and_then(Json::as_i64).unwrap_or(0),
                "tea_house": buildings.get("tea_house").and_then(Json::as_i64).unwrap_or(0),
                "bath_house": buildings.get("bath_house").and_then(Json::as_i64).unwrap_or(0),
                "training_centre": buildings.get("training_centre").and_then(Json::as_i64).unwrap_or(0),
                "golds": c_gold,
                "tokens": c_tokens,
            }))
        }
        ["upgrade", "max-members"] => {
            let mut tx = pool.begin().await?;
            let Some(m) = sqlx::query(
                "SELECT crew_id, role FROM crew_members WHERE character_id=? FOR UPDATE",
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
            let crew_id: u64 = m.get("crew_id");
            let c_row = sqlx::query("SELECT max_members, tokens FROM crews WHERE id=? FOR UPDATE")
                .bind(crew_id)
                .fetch_one(&mut *tx)
                .await?;
            let cur_max = i64::from(c_row.get::<i32, _>("max_members"));
            let c_tokens = i64::from(c_row.get::<i32, _>("tokens"));
            if cur_max >= 50 {
                return Ok(json!({ "errorMessage": "Already at max members" }));
            }
            let cost = cur_max * 500;
            if c_tokens < cost {
                return Ok(json!({ "errorMessage": "Not enough crew tokens" }));
            }
            let new_max = (cur_max + 5).min(50);
            sqlx::query("UPDATE crews SET max_members=?, tokens=?, updated_at=NOW() WHERE id=?")
                .bind(new_max)
                .bind(c_tokens - cost)
                .bind(crew_id)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            Ok(json!({ "success": true, "max_members": new_max }))
        }
        ["announcements"] => {
            let Some(m) =
                sqlx::query("SELECT crew_id, role FROM crew_members WHERE character_id=?")
                    .bind(char_id)
                    .fetch_optional(pool)
                    .await?
            else {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            };
            let role: String = m.get("role");
            if role != "master" && role != "elder" {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            }
            let ann = params.get("announcement").cloned().unwrap_or_default();
            sqlx::query("UPDATE crews SET announcement_draft=?, updated_at=NOW() WHERE id=?")
                .bind(ann)
                .bind(m.get::<u64, _>("crew_id"))
                .execute(pool)
                .await?;
            Ok(json!({ "success": true }))
        }
        ["announcement", "publish"] => {
            let Some(m) =
                sqlx::query("SELECT crew_id, role FROM crew_members WHERE character_id=?")
                    .bind(char_id)
                    .fetch_optional(pool)
                    .await?
            else {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            };
            let role: String = m.get("role");
            if role != "master" && role != "elder" {
                return Ok(json!({ "errorMessage": "Unauthorized" }));
            }
            sqlx::query(
                "UPDATE crews SET announcement_published = announcement_draft, updated_at=NOW() WHERE id=?",
            )
            .bind(m.get::<u64, _>("crew_id"))
            .execute(pool)
            .await?;
            Ok(json!("ok"))
        }
        ["player", "stamina", "upgrade-max"] => {
            let mut tx = pool.begin().await?;
            let Some(m) =
                sqlx::query("SELECT max_stamina FROM crew_members WHERE character_id=? FOR UPDATE")
                    .bind(char_id)
                    .fetch_optional(&mut *tx)
                    .await?
            else {
                return Ok(json!({ "errorMessage": "Not in a crew" }));
            };
            let cur_max = i64::from(m.get::<i32, _>("max_stamina"));
            if cur_max >= 300 {
                return Ok(json!({ "errorMessage": "Max stamina already at cap" }));
            }
            let tokens = i64::from(
                sqlx::query_scalar::<_, i32>("SELECT tokens FROM users WHERE id=? FOR UPDATE")
                    .bind(user_id)
                    .fetch_one(&mut *tx)
                    .await?,
            );
            if tokens < 500 {
                return Ok(json!({ "errorMessage": "Not enough tokens" }));
            }
            let new_max = (cur_max + 50).min(300);
            sqlx::query("UPDATE users SET tokens=?, updated_at=NOW() WHERE id=?")
                .bind(tokens - 500)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "UPDATE crew_members SET max_stamina=?, updated_at=NOW() WHERE character_id=?",
            )
            .bind(new_max)
            .bind(char_id)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            Ok(json!({ "status": "ok" }))
        }
        ["player", "boost-prestige"] => {
            let mut tx = pool.begin().await?;
            let in_crew: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM crew_members WHERE character_id=?")
                    .bind(char_id)
                    .fetch_one(&mut *tx)
                    .await?;
            if in_crew == 0 {
                return Ok(json!({ "errorMessage": "Not in a crew" }));
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
                "UPDATE crew_members SET prestige_boost_expires_at = DATE_ADD(NOW(), INTERVAL 86400 SECOND), updated_at=NOW() WHERE character_id=?",
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
            let mut tx = pool.begin().await?;
            let Some(m) = sqlx::query(
                "SELECT stamina, max_stamina FROM crew_members WHERE character_id=? FOR UPDATE",
            )
            .bind(char_id)
            .fetch_optional(&mut *tx)
            .await?
            else {
                return Ok(json!({ "errorMessage": "Not in a crew" }));
            };
            let st = i64::from(m.get::<i32, _>("stamina"));
            let max_st = i64::from(m.get::<i32, _>("max_stamina"));
            if st >= max_st {
                return Ok(json!({ "errorMessage": "Stamina is already full" }));
            }
            let restored = (max_st - st).min(50);
            let onigiri = sqlx::query(
                "SELECT id, quantity FROM character_items WHERE character_id=? AND item_id='material_69' FOR UPDATE",
            )
            .bind(char_id)
            .fetch_optional(&mut *tx)
            .await?;
            if let Some(o_row) = onigiri {
                let o_id: u64 = o_row.get("id");
                let qty = i64::from(o_row.get::<i32, _>("quantity"));
                if qty >= 1 {
                    let rem = qty - 1;
                    if rem == 0 {
                        sqlx::query("DELETE FROM character_items WHERE id=?")
                            .bind(o_id)
                            .execute(&mut *tx)
                            .await?;
                    } else {
                        sqlx::query(
                            "UPDATE character_items SET quantity=?, updated_at=NOW() WHERE id=?",
                        )
                        .bind(rem)
                        .bind(o_id)
                        .execute(&mut *tx)
                        .await?;
                    }
                    sqlx::query(
                        "UPDATE crew_members SET stamina = stamina + ?, updated_at=NOW() WHERE character_id=?",
                    )
                    .bind(restored)
                    .bind(char_id)
                    .execute(&mut *tx)
                    .await?;
                    tx.commit().await?;
                    return Ok(json!({
                        "status": "ok",
                        "data": {
                            "currency_type": 2,
                            "currency_remaining": rem,
                            "currency_used": 1,
                            "restored_stamina": restored,
                        }
                    }));
                }
            }
            let tokens = i64::from(
                sqlx::query_scalar::<_, i32>("SELECT tokens FROM users WHERE id=? FOR UPDATE")
                    .bind(user_id)
                    .fetch_one(&mut *tx)
                    .await?,
            );
            if tokens < 30 {
                return Ok(json!({ "errorMessage": "Not enough tokens or Golden Stamina Roll" }));
            }
            let rem_tokens = tokens - 30;
            sqlx::query("UPDATE users SET tokens=?, updated_at=NOW() WHERE id=?")
                .bind(rem_tokens)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "UPDATE crew_members SET stamina = stamina + ?, updated_at=NOW() WHERE character_id=?",
            )
            .bind(restored)
            .bind(char_id)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            Ok(json!({
                "status": "ok",
                "data": {
                    "currency_type": 1,
                    "currency_remaining": rem_tokens,
                    "currency_used": 30,
                    "restored_stamina": restored,
                }
            }))
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
        ["create"] => {
            let name = params.get("name").map(|s| s.trim()).unwrap_or("");
            if name.len() < 3 || name.len() > 50 {
                return Ok(json!({ "errorMessage": "Name must be 3-50 characters" }));
            }
            let in_crew: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM crew_members WHERE character_id=?")
                    .bind(char_id)
                    .fetch_one(pool)
                    .await?;
            if in_crew > 0 {
                return Ok(json!({ "errorMessage": "Already in a crew" }));
            }
            let taken: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM crews WHERE name=?")
                .bind(name)
                .fetch_one(pool)
                .await?;
            if taken > 0 {
                return Ok(json!({ "errorMessage": "Crew name already taken" }));
            }
            let (sid, _, _) = ensure_crew_season(pool).await?;
            let mut tx = pool.begin().await?;
            let tokens = i64::from(
                sqlx::query_scalar::<_, i32>("SELECT tokens FROM users WHERE id=? FOR UPDATE")
                    .bind(user_id)
                    .fetch_one(&mut *tx)
                    .await?,
            );
            if tokens < 1000 {
                return Ok(json!({ "errorMessage": "Not enough tokens" }));
            }
            sqlx::query("UPDATE users SET tokens=?, updated_at=NOW() WHERE id=?")
                .bind(tokens - 1000)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            let res = sqlx::query(
                "INSERT INTO crews (season_id, name, master_id, max_members, buildings, created_at, updated_at) \
                 VALUES (?, ?, ?, 20, '{\"kushi_dango\":0,\"tea_house\":0,\"bath_house\":0,\"training_centre\":0}', NOW(), NOW())",
            )
            .bind(sid)
            .bind(name)
            .bind(char_id)
            .execute(&mut *tx)
            .await?;
            let crew_id = res.last_insert_id();
            sqlx::query(
                "INSERT INTO crew_members (crew_id, character_id, role, battle_role, stamina, max_stamina, created_at, updated_at) \
                 VALUES (?, ?, 'master', 1, 50, 50, NOW(), NOW())",
            )
            .bind(crew_id)
            .bind(char_id)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            Ok(json!({ "status": "ok" }))
        }
        ["rename"] => {
            let name = params.get("name").map(|s| s.trim()).unwrap_or("");
            if name.len() < 3 || name.len() > 50 {
                return Ok(json!({ "errorMessage": "Name must be 3-50 characters" }));
            }
            let Some(m) =
                sqlx::query("SELECT crew_id, role FROM crew_members WHERE character_id=?")
                    .bind(char_id)
                    .fetch_optional(pool)
                    .await?
            else {
                return Ok(json!({ "errorMessage": "Only master can rename crew" }));
            };
            if m.get::<String, _>("role") != "master" {
                return Ok(json!({ "errorMessage": "Only master can rename crew" }));
            }
            let crew_id: u64 = m.get("crew_id");
            let taken: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM crews WHERE name=? AND id!=?")
                    .bind(name)
                    .bind(crew_id)
                    .fetch_one(pool)
                    .await?;
            if taken > 0 {
                return Ok(json!({ "errorMessage": "Crew name already taken" }));
            }
            sqlx::query("UPDATE crews SET name=?, updated_at=NOW() WHERE id=?")
                .bind(name)
                .bind(crew_id)
                .execute(pool)
                .await?;
            Ok(json!("ok"))
        }
        ["season-histories"] => {
            let rows = sqlx::query(
                "SELECT id, number, DATE_FORMAT(started_at, '%Y-%m-%dT%H:%i:%sZ') AS started_at, \
                 DATE_FORMAT(ended_at, '%Y-%m-%dT%H:%i:%sZ') AS ended_at \
                 FROM crew_seasons ORDER BY number DESC LIMIT 10",
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
        ["battle", "opponents"] => {
            let crew_id: u64 =
                sqlx::query_scalar("SELECT crew_id FROM crew_members WHERE character_id=?")
                    .bind(char_id)
                    .fetch_optional(pool)
                    .await?
                    .unwrap_or(0);
            let crews =
                format_crew_summaries(pool, "WHERE c.id != ?", Some(crew_id.to_string()), 10)
                    .await?;
            Ok(json!({ "crews": crews }))
        }
        ["battle", "opponents", id_str] | ["request", "available", id_str] => {
            let crews =
                format_crew_summaries(pool, "WHERE c.id = ?", Some((*id_str).into()), 1).await?;
            Ok(json!({ "crews": crews }))
        }
        ["battle", "castles"] => {
            let (sid, _, _) = ensure_crew_season(pool).await?;
            let rows = ensure_castles_exist(pool, sid).await?;
            let mut castles = Vec::with_capacity(rows.len());
            for r in rows {
                let idx = r.get::<u8, _>("castle_index") as usize;
                let owner_id = r.get::<Option<u64>, _>("owner_crew_id");
                let owner_name = if let Some(oid) = owner_id {
                    sqlx::query_scalar::<_, String>("SELECT name FROM crews WHERE id=?")
                        .bind(oid)
                        .fetch_optional(pool)
                        .await?
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                castles.push(json!({
                    "id": r.get::<u64, _>("id"),
                    "name": CASTLE_NAMES.get(idx).copied().unwrap_or("Castle"),
                    "owner_id": owner_id,
                    "owner_name": owner_name,
                    "wall_hp": r.get::<u32, _>("wall_hp"),
                    "defender_hp": r.get::<u32, _>("defender_hp"),
                }));
            }
            Ok(json!({ "castles": castles, "a": "" }))
        }
        ["battle", "castles", cid_str] => {
            let cid = cid_str.parse::<u64>().unwrap_or(0);
            let Some(r) = sqlx::query("SELECT * FROM crew_castles WHERE id=?")
                .bind(cid)
                .fetch_optional(pool)
                .await?
            else {
                return Ok(json!({ "castles": [] }));
            };
            let idx = r.get::<u8, _>("castle_index") as usize;
            let owner_id = r.get::<Option<u64>, _>("owner_crew_id");
            let owner_name = if let Some(oid) = owner_id {
                sqlx::query_scalar::<_, String>("SELECT name FROM crews WHERE id=?")
                    .bind(oid)
                    .fetch_optional(pool)
                    .await?
                    .unwrap_or_default()
            } else {
                String::new()
            };
            Ok(json!({
                "castles": [{
                    "id": r.get::<u64, _>("id"),
                    "name": CASTLE_NAMES.get(idx).copied().unwrap_or("Castle"),
                    "owner_id": owner_id,
                    "owner_name": owner_name,
                    "wall_hp": r.get::<u32, _>("wall_hp"),
                    "defender_hp": r.get::<u32, _>("defender_hp"),
                }]
            }))
        }
        ["battle", "castles", _, "ranks"] => Ok(json!({ "ranks": [] })),
        ["battle", "castles", cid_str, "recovery"] => {
            let cid = cid_str.parse::<u64>().unwrap_or(0);
            let Some(r) = sqlx::query(
                "SELECT UNIX_TIMESTAMP(last_recovery_at) AS last_ts FROM crew_castles WHERE id=?",
            )
            .bind(cid)
            .fetch_optional(pool)
            .await?
            else {
                return Ok(json!({ "errorMessage": "Castle not found" }));
            };
            let last_ts = r
                .try_get::<Option<i64>, _>("last_ts")
                .ok()
                .flatten()
                .unwrap_or(0);
            let now = now_ts();
            let next_rec = if last_ts > 0 {
                (last_ts + 1800 - now).max(0)
            } else {
                0
            };
            let tokens: i32 = sqlx::query_scalar("SELECT tokens FROM users WHERE id=?")
                .bind(user_id)
                .fetch_optional(pool)
                .await?
                .unwrap_or(0);
            Ok(json!({
                "n": now + next_rec,
                "t": tokens,
                "a": 5,
                "f": "",
            }))
        }
        ["battle", "castles", cid_str, "recover"] => {
            let cid = cid_str.parse::<u64>().unwrap_or(0);
            let Some(crew_id) = sqlx::query_scalar::<_, u64>(
                "SELECT crew_id FROM crew_members WHERE character_id=?",
            )
            .bind(char_id)
            .fetch_optional(pool)
            .await?
            else {
                return Ok(json!({ "errorMessage": "Castle not found" }));
            };
            let Some(c_row) = sqlx::query(
                "SELECT owner_crew_id, wall_hp, UNIX_TIMESTAMP(last_recovery_at) AS last_ts FROM crew_castles WHERE id=?",
            )
            .bind(cid)
            .fetch_optional(pool)
            .await?
            else {
                return Ok(json!({ "errorMessage": "Castle not found" }));
            };
            if c_row.get::<Option<u64>, _>("owner_crew_id") != Some(crew_id) {
                return Ok(json!({ "errorMessage": "You do not own this castle" }));
            }
            let last_ts = c_row
                .try_get::<Option<i64>, _>("last_ts")
                .ok()
                .flatten()
                .unwrap_or(0);
            let now = now_ts();
            if last_ts > 0 && now - last_ts < 1800 {
                let rem_min = ((1800 - (now - last_ts)) / 60).max(1);
                return Ok(json!({
                    "errorMessage": format!("Recovery available in {rem_min} minutes"),
                    "code": 429,
                }));
            }
            let wall_hp = c_row.get::<u32, _>("wall_hp");
            let new_hp = (wall_hp + 5).min(100);
            sqlx::query(
                "UPDATE crew_castles SET wall_hp=?, last_recovery_at=NOW(), updated_at=NOW() WHERE id=?",
            )
            .bind(new_hp)
            .bind(cid)
            .execute(pool)
            .await?;
            let tokens: i32 = sqlx::query_scalar("SELECT tokens FROM users WHERE id=?")
                .bind(user_id)
                .fetch_optional(pool)
                .await?
                .unwrap_or(0);
            Ok(json!({
                "n": now + 1800,
                "t": tokens,
                "a": 5,
                "f": "",
            }))
        }
        ["battle", "castles", cid_str, "defenders"] => {
            let cid = cid_str.parse::<u64>().unwrap_or(0);
            let owner_id: Option<u64> =
                sqlx::query_scalar("SELECT owner_crew_id FROM crew_castles WHERE id=?")
                    .bind(cid)
                    .fetch_optional(pool)
                    .await?
                    .flatten();
            let Some(oid) = owner_id else {
                return Ok(json!({ "defenders": [] }));
            };
            let defenders = format_crew_members(pool, oid, Some(2)).await?;
            Ok(json!({ "defenders": defenders }))
        }
        ["battle", "role", "switch", _] => {
            let Some(m) = sqlx::query("SELECT battle_role FROM crew_members WHERE character_id=?")
                .bind(char_id)
                .fetch_optional(pool)
                .await?
            else {
                return Ok(json!({ "errorMessage": "Not in a crew" }));
            };
            let cur_role: u8 = m.get("battle_role");
            let new_role: u8 = if cur_role == 1 { 2 } else { 1 };
            sqlx::query(
                "UPDATE crew_members SET battle_role=?, role_limit_at='', updated_at=NOW() WHERE character_id=?",
            )
            .bind(new_role)
            .bind(char_id)
            .execute(pool)
            .await?;
            Ok(json!({ "success": true, "role": new_role }))
        }
        ["battle", "attackers"] => {
            let Some(crew_id) = sqlx::query_scalar::<_, u64>(
                "SELECT crew_id FROM crew_members WHERE character_id=?",
            )
            .bind(char_id)
            .fetch_optional(pool)
            .await?
            else {
                return Ok(json!({ "members": [] }));
            };
            let members = format_crew_members(pool, crew_id, Some(1)).await?;
            Ok(json!({ "members": members }))
        }
        ["battle", "phase1", "start"] => start_phase_one(pool, char_id, params).await,
        ["battle", "phase1", "finish"] => finish_phase_one(pool, char_id, params).await,
        ["battle", "phase2", "start"] => {
            if !signed_battle_request(char_id, params, false) {
                return Ok(json!({"errorMessage":"Invalid battle request","code":422}));
            }
            let c_input = params
                .get("c")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            let (sid, _, phase) = ensure_crew_season(pool).await?;
            if phase != 2 {
                return Ok(
                    json!({"errorMessage":"The crew season is not in phase two","code":409}),
                );
            }
            let _ = ensure_castles_exist(pool, sid).await?;
            let mut tx = pool.begin().await?;
            let Some(m) = sqlx::query(
                "SELECT crew_id, stamina FROM crew_members WHERE character_id=? FOR UPDATE",
            )
            .bind(char_id)
            .fetch_optional(&mut *tx)
            .await?
            else {
                return Ok(json!({ "errorMessage": "Not in a crew", "code": 400 }));
            };
            let att_crew_id: u64 = m.get("crew_id");
            let stamina = m.get::<i32, _>("stamina");
            let nonce = params.get("b").map(String::as_str).unwrap_or("");
            if let Some(saved)=sqlx::query_scalar::<_,Json>("SELECT battle_data FROM crew_battles WHERE season_id=? AND JSON_UNQUOTE(JSON_EXTRACT(battle_data,'$.character_id'))=? AND JSON_UNQUOTE(JSON_EXTRACT(battle_data,'$.request_nonce'))=? AND JSON_EXTRACT(battle_data,'$.phase')=2 ORDER BY id DESC LIMIT 1").bind(sid).bind(char_id.to_string()).bind(nonce).fetch_optional(&mut *tx).await? {return Ok(saved["result"].clone());}

            if stamina < 10 {
                return Ok(json!({ "errorMessage": "Not enough stamina", "code": 402 }));
            }
            let castle_row = sqlx::query(
                "SELECT id, owner_crew_id, wall_hp FROM crew_castles WHERE id=? AND season_id=? LIMIT 1 FOR UPDATE",
            )
            .bind(c_input)
            .bind(sid)
            .fetch_optional(&mut *tx)
            .await?;
            let Some(c_row) = castle_row else {
                return Ok(json!({ "errorMessage": "Castle not found", "code": 404 }));
            };
            let castle_id: u64 = c_row.get("id");
            let owner_crew_id = c_row.get::<Option<u64>, _>("owner_crew_id");
            if owner_crew_id == Some(att_crew_id) {
                return Ok(json!({
                    "errorMessage": "The castle has been taken over by your crew.",
                    "code": 406,
                }));
            }
            let wall_hp = i64::from(c_row.get::<u32, _>("wall_hp"));
            let damage = rand_range(5, 20);
            let won = (wall_hp - damage) <= 0;
            let merit = if won {
                rand_range(50, 150)
            } else {
                rand_range(10, 50)
            };
            let new_stamina = stamina - 10;

            let att_name: String = sqlx::query_scalar("SELECT name FROM crews WHERE id=?")
                .bind(att_crew_id)
                .fetch_one(&mut *tx)
                .await?;
            let def_name: String = if let Some(def_id) = owner_crew_id {
                sqlx::query_scalar("SELECT name FROM crews WHERE id=?")
                    .bind(def_id)
                    .fetch_optional(&mut *tx)
                    .await?
                    .unwrap_or_else(|| "Neutral".into())
            } else {
                "Neutral".into()
            };

            sqlx::query("UPDATE crew_members SET stamina=?, updated_at=NOW() WHERE character_id=?")
                .bind(new_stamina)
                .bind(char_id)
                .execute(&mut *tx)
                .await?;
            if won {
                sqlx::query(
                    "UPDATE crew_castles SET owner_crew_id=?, wall_hp=100, defender_hp=100, updated_at=NOW() WHERE id=?",
                )
                .bind(att_crew_id)
                .bind(castle_id)
                .execute(&mut *tx)
                .await?;
                sqlx::query(
                    "UPDATE crews SET prestige = prestige + 10, updated_at=NOW() WHERE id=?",
                )
                .bind(att_crew_id)
                .execute(&mut *tx)
                .await?;
                if let Some(def_id) = owner_crew_id {
                    sqlx::query(
                        "UPDATE crews SET prestige = GREATEST(0, prestige - 5), updated_at=NOW() WHERE id=?",
                    )
                    .bind(def_id)
                    .execute(&mut *tx)
                    .await?;
                }
            } else {
                sqlx::query("UPDATE crew_castles SET wall_hp=?, updated_at=NOW() WHERE id=?")
                    .bind((wall_hp - damage).max(0))
                    .bind(castle_id)
                    .execute(&mut *tx)
                    .await?;
            }
            let result = json!({"b":true,"w":if won {&att_name}else{&def_name},"l":if won {&def_name}else{&att_name},"d":damage,"m":merit,"s":new_stamina});
            let data = json!({"phase":2,"character_id":char_id,"request_nonce":nonce,"completed":true,"merit":merit,"damage":damage,"result":result});
            sqlx::query("INSERT INTO crew_battles(season_id,castle_id,attacker_crew_id,defender_crew_id,attacker_won,battle_data,created_at,updated_at) VALUES(?,?,?,?,?,?,NOW(),NOW())").bind(sid).bind(castle_id).bind(att_crew_id).bind(owner_crew_id).bind(won).bind(data).execute(&mut *tx).await?;
            tx.commit().await?;

            Ok(result)
        }
        ["battle", "phase2", "finish"] => phase_two_result(pool, char_id, params).await,
        ["player", "minigame"] => {
            let energy: i32 =
                sqlx::query_scalar("SELECT minigame_energy FROM crew_members WHERE character_id=?")
                    .bind(char_id)
                    .fetch_optional(pool)
                    .await?
                    .unwrap_or(0);
            Ok(json!({ "energy": energy }))
        }
        ["player", "minigame", "start"] => {
            let Some(energy) = sqlx::query_scalar::<_, i32>(
                "SELECT minigame_energy FROM crew_members WHERE character_id=?",
            )
            .bind(char_id)
            .fetch_optional(pool)
            .await?
            else {
                return Ok(json!({ "errorMessage": "Not in a crew" }));
            };
            if energy <= 0 {
                return Ok(json!({ "errorMessage": "No energy left" }));
            }
            let code = auth::random_hex().unwrap_or_else(|_| "0123456789abcdef".into());
            Ok(json!({
                "c": code,
                "t": now_ts(),
            }))
        }
        ["player", "minigame", "finish"] => {
            let result = params
                .get("r")
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            let mut tx = pool.begin().await?;
            let Some(energy) = sqlx::query_scalar::<_, i32>(
                "SELECT minigame_energy FROM crew_members WHERE character_id=? FOR UPDATE",
            )
            .bind(char_id)
            .fetch_optional(&mut *tx)
            .await?
            else {
                return Ok(json!({ "errorMessage": "Not in a crew" }));
            };
            if energy > 0 {
                sqlx::query(
                    "UPDATE crew_members SET minigame_energy = minigame_energy - 1, updated_at=NOW() WHERE character_id=?",
                )
                .bind(char_id)
                .execute(&mut *tx)
                .await?;
            }
            if result != 1 {
                tx.commit().await?;
                return Ok(json!({ "errorMessage": "Game not completed" }));
            }
            let item_id = if rand_range(0, 1) == 0 {
                "material_939"
            } else {
                "material_941"
            };
            let qty = rand_range(1, 3);
            rewards::give_item(&mut tx, char_id, item_id, qty).await?;
            tx.commit().await?;
            Ok(json!({
                "r": [{ "id": item_id, "amount": qty }]
            }))
        }
        ["player", "minigame", "buy", _] => {
            let mut tx = pool.begin().await?;
            let tokens = i64::from(
                sqlx::query_scalar::<_, i32>("SELECT tokens FROM users WHERE id=? FOR UPDATE")
                    .bind(user_id)
                    .fetch_one(&mut *tx)
                    .await?,
            );
            if tokens < 5 {
                return Ok(json!({ "errorMessage": "Not enough tokens" }));
            }
            let new_tokens = tokens - 5;
            sqlx::query("UPDATE users SET tokens=?, updated_at=NOW() WHERE id=?")
                .bind(new_tokens)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            Ok(json!({ "t": new_tokens }))
        }
        _ => Ok(json!({ "errorMessage": format!("Unknown crew route: {path}") })),
    }
}
