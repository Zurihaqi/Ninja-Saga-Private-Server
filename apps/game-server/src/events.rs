//! Event catalog, PackageEvent, Giveaway, Leaderboard, and PvP AMF services.
//!
//! Source:
//! - `app/Services/Amf/EventsService.php`
//! - `app/Services/Amf/PackageEventService.php`
//! - `app/Services/Amf/GiveawayService.php`
//! - `app/Services/Amf/LeaderboardService.php`
//! - `app/Services/Amf/PvPService.php`
use crate::{
    account::{guard, integer, session_error, string},
    rewards,
};
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};
use std::time::{SystemTime, UNIX_EPOCH};

type Result = std::result::Result<Json, sqlx::Error>;

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Mirrors `EventsService::executeService`.
pub async fn execute_service(pool: &MySqlPool, args: &[Json]) -> Result {
    let sub = string(args, 0);
    let params = args
        .get(1)
        .and_then(Json::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    if sub == "get" {
        return get(pool, params).await;
    }
    if let Some((id, key)) = guard_pair(params)
        && let Some(error) = guard(pool, id, &key, true).await?
    {
        return Ok(error);
    }
    let row =
        sqlx::query("SELECT data FROM game_events WHERE panel=? AND active=1 ORDER BY id LIMIT 1")
            .bind(&sub)
            .fetch_optional(pool)
            .await?;
    let Some(row) = row else {
        return Ok(json!({
            "status": 0,
            "error": format!("Event '{sub}' is not currently active.")
        }));
    };
    let mut out = serde_json::Map::new();
    out.insert("status".into(), json!(1));
    out.insert("error".into(), json!(0));
    if let Some(Json::Object(map)) = row.try_get::<Option<Json>, _>("data").ok().flatten() {
        for (k, v) in map {
            out.insert(k, v);
        }
    }
    Ok(Json::Object(out))
}

/// Mirrors `EventsService::get`, called during client boot and by event panels.
/// `params` may be null (boot) or a `[characterId, sessionKey]` pair.
pub async fn get(pool: &MySqlPool, args: &[Json]) -> Result {
    if let Some((id, key)) = guard_pair(args)
        && let Some(error) = guard(pool, id, &key, true).await?
    {
        return Ok(error);
    }
    let rows = sqlx::query(
        "SELECT title,type,panel,date,icon,inside,image_url,description,data \
         FROM game_events WHERE active=1 ORDER BY sort_order,id",
    )
    .fetch_all(pool)
    .await?;
    let mut seasonal = Vec::new();
    let mut permanent = Vec::new();
    let mut features = Vec::new();
    let mut package: Option<Json> = None;
    for row in rows.iter() {
        let title: String = row.get("title");
        let event_type: String = row.get("type");
        match event_type.as_str() {
            "seasonal" => seasonal.push(json!({
                "name": title,
                "desc": row.get::<Option<String>, _>("description"),
                "date": row.get::<Option<String>, _>("date"),
                "img": row.get::<Option<String>, _>("image_url"),
                "panel": row.get::<Option<String>, _>("panel"),
            })),
            "permanent" => permanent.push(json!({
                "name": title,
                "icon": icon(row),
                "panel": row.get::<Option<String>, _>("panel"),
            })),
            "feature" => {
                let mut entry = json!({
                    "name": title,
                    "icon": icon(row),
                    "panel": row.get::<Option<String>, _>("panel"),
                });
                if row.try_get::<bool, _>("inside")? {
                    entry["inside"] = json!(true);
                }
                features.push(entry);
            }
            "package" => {
                // Only the first active package (lowest sort_order/id) is surfaced.
                if package.is_none() {
                    let content = row
                        .try_get::<Option<Json>, _>("data")
                        .ok()
                        .flatten()
                        .and_then(|data| data.get("content").cloned())
                        .unwrap_or(json!([]));
                    package = Some(json!({
                        "name": title,
                        "date": row.get::<Option<String>, _>("date"),
                        "content": content,
                    }));
                }
            }
            _ => {}
        }
    }
    Ok(json!({
        "status": 1,
        "error": 0,
        "events": {
            "seasonal": seasonal,
            "event:permanent": permanent,
            "features": features,
            "packages": package,
        }
    }))
}

pub async fn dispatch_package_event(pool: &MySqlPool, method: &str, args: &[Json]) -> Result {
    match method {
        "executeService" => {
            let sub = string(args, 0);
            let params = args
                .get(1)
                .and_then(Json::as_array)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            match sub.as_str() {
                "buyChuninPackage" => buy_chunin_package(pool, params).await,
                "buyDesignContest" => buy_design_contest(pool, params).await,
                _ => Ok(session_error(&format!(
                    "PackageEvent sub-service '{sub}' is not implemented"
                ))),
            }
        }
        "buyChuninPackage" => buy_chunin_package(pool, args).await,
        "buyDesignContest" => buy_design_contest(pool, args).await,
        _ => Ok(session_error("Invalid PackageEvent method")),
    }
}

async fn buy_chunin_package(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let cfg: Option<Json> =
        sqlx::query_scalar("SELECT `value` FROM game_configs WHERE `key`='chunin_package'")
            .fetch_optional(pool)
            .await?;
    let cfg = cfg.unwrap_or_else(|| {
        json!({
            "cost": 1500,
            "rewards": [
                {"type": "skill", "id": "skill_2000"},
                {"type": "item",  "id": "set_2000_%s"},
                {"type": "item",  "id": "hair_2000_%s"},
                {"type": "item",  "id": "wpn_2000"}
            ]
        })
    });
    let cost = cfg.get("cost").and_then(Json::as_i64).unwrap_or(1500);
    let rewards_cfg = cfg
        .get("rewards")
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_default();

    let mut tx = pool.begin().await?;
    let Some(c_row) = sqlx::query("SELECT user_id, gender FROM characters WHERE id=? FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
    else {
        return Ok(session_error("Character not found"));
    };
    let user_id: u64 = c_row.get("user_id");
    let gender: i8 = c_row.get("gender");

    for r in &rewards_cfg {
        let r_type = r.get("type").and_then(Json::as_str).unwrap_or("");
        let r_id = r
            .get("id")
            .and_then(Json::as_str)
            .unwrap_or("")
            .replace("%s", &gender.to_string());
        if r_type == "skill" {
            let owned: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM character_skills WHERE character_id=? AND skill_id=?",
            )
            .bind(id)
            .bind(&r_id)
            .fetch_one(&mut *tx)
            .await?;
            if owned > 0 {
                return Ok(json!({
                    "status": 2,
                    "result": "You already own items from this package!"
                }));
            }
        } else if r_type == "item" {
            let owned: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM character_items WHERE character_id=? AND item_id=?",
            )
            .bind(id)
            .bind(&r_id)
            .fetch_one(&mut *tx)
            .await?;
            if owned > 0 {
                return Ok(json!({
                    "status": 2,
                    "result": "You already own items from this package!"
                }));
            }
        }
    }

    let tokens = i64::from(
        sqlx::query_scalar::<_, i32>("SELECT tokens FROM users WHERE id=? FOR UPDATE")
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?,
    );
    if tokens < cost {
        return Ok(json!({"status": 2, "result": "Not enough tokens!"}));
    }
    let new_tokens = tokens - cost;
    sqlx::query("UPDATE users SET tokens=?, updated_at=NOW() WHERE id=?")
        .bind(new_tokens)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    for r in &rewards_cfg {
        let r_type = r.get("type").and_then(Json::as_str).unwrap_or("");
        let r_id = r
            .get("id")
            .and_then(Json::as_str)
            .unwrap_or("")
            .replace("%s", &gender.to_string());
        if r_type == "skill" {
            sqlx::query(
                "INSERT INTO character_skills (character_id, skill_id, created_at, updated_at) VALUES (?, ?, NOW(), NOW())",
            )
            .bind(id)
            .bind(&r_id)
            .execute(&mut *tx)
            .await?;
        } else if r_type == "item" {
            rewards::give_item(&mut tx, id, &r_id, 1).await?;
        }
    }
    tx.commit().await?;
    Ok(json!({
        "status": 1,
        "result": "Package purchased successfully!",
        "user_tokens": new_tokens,
    }))
}

async fn buy_design_contest(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let item_id = string(args, 2);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(item_row) = sqlx::query("SELECT price_gold, price_tokens FROM items WHERE item_id=?")
        .bind(&item_id)
        .fetch_optional(pool)
        .await?
    else {
        return Ok(session_error("Item not found in database"));
    };
    let price_gold = i64::from(item_row.get::<i32, _>("price_gold"));
    let price_tokens = i64::from(item_row.get::<i32, _>("price_tokens"));

    let mut tx = pool.begin().await?;
    let Some(c_row) = sqlx::query("SELECT user_id, gold FROM characters WHERE id=? FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
    else {
        return Ok(session_error("Character not found"));
    };
    let user_id: u64 = c_row.get("user_id");
    let gold = i64::from(c_row.get::<i32, _>("gold"));

    if price_tokens > 0 {
        let tokens = i64::from(
            sqlx::query_scalar::<_, i32>("SELECT tokens FROM users WHERE id=? FOR UPDATE")
                .bind(user_id)
                .fetch_one(&mut *tx)
                .await?,
        );
        if tokens < price_tokens {
            return Ok(json!({"status": 2, "result": "Not enough tokens!"}));
        }
        sqlx::query("UPDATE users SET tokens=?, updated_at=NOW() WHERE id=?")
            .bind(tokens - price_tokens)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
    } else if price_gold > 0 {
        if gold < price_gold {
            return Ok(json!({"status": 2, "result": "Not enough gold!"}));
        }
        sqlx::query("UPDATE characters SET gold=?, updated_at=NOW() WHERE id=?")
            .bind(gold - price_gold)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }

    rewards::give_item(&mut tx, id, &item_id, 1).await?;
    tx.commit().await?;
    Ok(json!({
        "status": 1,
        "result": "Item bought successfully!"
    }))
}

pub async fn dispatch_giveaway(pool: &MySqlPool, method: &str, args: &[Json]) -> Result {
    match method {
        "get" => giveaway_get(pool, args).await,
        "history" => giveaway_history(pool, args).await,
        "participate" => giveaway_participate(pool, args).await,
        _ => Ok(session_error("Invalid Giveaway method")),
    }
}

async fn giveaway_get(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let rows = sqlx::query(
        "SELECT id, title, description, prizes, requirements, UNIX_TIMESTAMP(ends_at) AS end_ts, \
         (ends_at > NOW()) AS is_active, winners \
         FROM giveaways \
         WHERE ends_at > NOW() \
            OR (ends_at <= NOW() AND ends_at >= DATE_SUB(NOW(), INTERVAL 30 DAY) AND winners IS NOT NULL AND JSON_LENGTH(winners) > 0) \
         ORDER BY ends_at ASC",
    )
    .fetch_all(pool)
    .await?;

    let now = now_unix();
    let mut giveaways = Vec::with_capacity(rows.len());
    for row in rows {
        let gid: u64 = row.get("id");
        let is_active: i64 = row.try_get("is_active").unwrap_or(0);
        let end_ts: i64 = row.try_get("end_ts").unwrap_or(now);
        let prizes = row
            .try_get::<Option<Json>, _>("prizes")
            .ok()
            .flatten()
            .unwrap_or_else(|| json!([]));
        let reqs_raw = row
            .try_get::<Option<Json>, _>("requirements")
            .ok()
            .flatten()
            .unwrap_or_else(|| json!([]));
        let winners_raw = row
            .try_get::<Option<Json>, _>("winners")
            .ok()
            .flatten()
            .unwrap_or_else(|| json!([]));

        let participants: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM giveaway_participants WHERE giveaway_id=?")
                .bind(gid)
                .fetch_one(pool)
                .await?;
        let joined_cnt: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM giveaway_participants WHERE giveaway_id=? AND character_id=?",
        )
        .bind(gid)
        .bind(id)
        .fetch_one(pool)
        .await?;

        let winners = if is_active == 0 {
            winners_raw
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|w| {
                            json!({
                                "id": w.get("id").and_then(Json::as_i64).unwrap_or(0),
                                "name": w.get("name").and_then(Json::as_str).unwrap_or("Unknown"),
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        giveaways.push(json!({
            "id": gid,
            "title": row.get::<String, _>("title"),
            "desc": row.get::<Option<String>, _>("description").unwrap_or_default(),
            "prizes": prizes,
            "participants": participants,
            "timestamp": if is_active != 0 { (end_ts - now).max(0) } else { 0 },
            "joined": joined_cnt > 0,
            "requirements": format_giveaway_requirements(&reqs_raw),
            "winners": winners,
        }));
    }

    Ok(json!({
        "status": 1,
        "error": 0,
        "giveaways": giveaways,
    }))
}

async fn giveaway_history(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let rows = sqlx::query(
        "SELECT title, description, DATE_FORMAT(ends_at, '%Y-%m-%d %H:%i') AS ended_at, prizes \
         FROM giveaways WHERE ends_at <= NOW() ORDER BY ends_at DESC LIMIT 20",
    )
    .fetch_all(pool)
    .await?;

    let list: Vec<Json> = rows
        .into_iter()
        .map(|row| {
            let prizes = row
                .try_get::<Option<Json>, _>("prizes")
                .ok()
                .flatten()
                .unwrap_or_else(|| json!([]));
            json!({
                "title": row.get::<String, _>("title"),
                "description": row.get::<Option<String>, _>("description").unwrap_or_default(),
                "ended_at": row.get::<Option<String>, _>("ended_at").unwrap_or_default(),
                "prizes": prizes,
            })
        })
        .collect();

    Ok(json!({
        "status": 1,
        "giveaway": list,
    }))
}

async fn giveaway_participate(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let giveaway_id = integer(args, 2);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(g_row) = sqlx::query(
        "SELECT requirements, (ends_at > NOW()) AS is_active, winners FROM giveaways WHERE id=?",
    )
    .bind(giveaway_id)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(json!({"status": 2, "result": "Giveaway not found."}));
    };
    let is_active: i64 = g_row.try_get("is_active").unwrap_or(0);
    let winners_non_empty = g_row
        .try_get::<Option<Json>, _>("winners")
        .ok()
        .flatten()
        .and_then(|v| v.as_array().map(|a| !a.is_empty()))
        .unwrap_or(false);
    if is_active == 0 || winners_non_empty {
        return Ok(json!({"status": 2, "result": "This giveaway has already ended."}));
    }

    let already: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM giveaway_participants WHERE giveaway_id=? AND character_id=?",
    )
    .bind(giveaway_id)
    .bind(id)
    .fetch_one(pool)
    .await?;
    if already > 0 {
        return Ok(json!({"status": 2, "result": "You have already joined this giveaway."}));
    }

    let Some(c_row) = sqlx::query(
        "SELECT c.level, c.rank, c.pvp_played, c.pvp_won, u.account_type \
         FROM characters c JOIN users u ON u.id = c.user_id WHERE c.id=?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(session_error("Character not found"));
    };
    let reqs_raw = g_row
        .try_get::<Option<Json>, _>("requirements")
        .ok()
        .flatten()
        .unwrap_or_else(|| json!([]));
    if let Some(err_msg) = check_giveaway_requirements(&reqs_raw, &c_row) {
        return Ok(json!({"status": 2, "result": err_msg}));
    }

    sqlx::query(
        "INSERT INTO giveaway_participants (giveaway_id, character_id, created_at, updated_at) VALUES (?, ?, NOW(), NOW())",
    )
    .bind(giveaway_id)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(json!({"status": 1, "error": 0}))
}

fn rank_label(rank: i64) -> &'static str {
    match rank {
        1 => "Genin",
        3 => "Chunin",
        5 => "Tensai Chunin",
        6 => "Jounin",
        7 => "Tensai Jounin",
        8 => "Special Jounin",
        9 => "Tensai Special Jounin",
        10 => "Ninja Tutor",
        11 => "Senior Ninja Tutor",
        _ => "Genin",
    }
}

fn format_giveaway_requirements(reqs: &Json) -> Vec<String> {
    let Some(arr) = reqs.as_array() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for req in arr {
        let r_type = req.get("type").and_then(Json::as_str).unwrap_or("");
        let val = req.get("value").and_then(Json::as_i64).unwrap_or(0);
        match r_type {
            "level" => out.push(format!("Minimum Level {val}")),
            "pvp_battles" => out.push(format!("Minimum {val} PvP Battles Played")),
            "pvp_wins" => out.push(format!("Minimum {val} PvP Wins")),
            "rank" => out.push(format!("Minimum Rank: {}", rank_label(val))),
            "emblem" => out.push("Emblem User Required".into()),
            _ => {}
        }
    }
    out
}

fn check_giveaway_requirements(reqs: &Json, row: &sqlx::mysql::MySqlRow) -> Option<String> {
    let arr = reqs.as_array()?;
    let level = i64::from(row.get::<i32, _>("level"));
    let rank = i64::from(row.get::<i8, _>("rank"));
    let pvp_played = i64::from(row.get::<i32, _>("pvp_played"));
    let pvp_won = i64::from(row.get::<i32, _>("pvp_won"));
    let account_type = i64::from(row.get::<i8, _>("account_type"));
    for req in arr {
        let r_type = req.get("type").and_then(Json::as_str).unwrap_or("");
        let val = req.get("value").and_then(Json::as_i64).unwrap_or(0);
        match r_type {
            "level" if level < val => return Some(format!("Requires Level {val}.")),
            "pvp_battles" if pvp_played < val => {
                return Some(format!("Requires at least {val} PvP battles played."));
            }
            "pvp_wins" if pvp_won < val => {
                return Some(format!("Requires at least {val} PvP wins."));
            }
            "rank" if rank < val => {
                return Some(format!("Requires Rank: {}.", rank_label(val)));
            }
            "emblem" if account_type < 1 => {
                return Some("Requires Emblem User status.".into());
            }
            _ => {}
        }
    }
    None
}

/// Mirrors `LeaderboardService::getData`.
pub async fn leaderboard_get_data(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let category = string(args, 2);
    let cat_lower = category.to_lowercase();
    let event_key = match cat_lower.as_str() {
        "christmas_2025" | "kill" | "kills" | "" => "christmas_2025",
        "thanksgiving_2025" => "thanksgiving_2025",
        "confronting_death_2025" => "confronting_death_2025",
        "halloween_2025" => "halloween_2025",
        "monster_hunter" => "monster_hunter",
        "dragon_hunt" => "dragon_hunt",
        other => other,
    };

    let rows = sqlx::query(
        "SELECT ced.battles, c.name, c.gender, c.hair_style, c.hair_color, c.skin_color, \
                c.equipment_weapon, c.equipment_back, c.equipment_clothing, c.equipment_accessory \
         FROM character_event_data ced \
         JOIN characters c ON c.id = ced.character_id \
         WHERE ced.event_key = ? AND ced.battles > 0 \
         ORDER BY ced.battles DESC LIMIT 100",
    )
    .bind(event_key)
    .fetch_all(pool)
    .await?;

    let mut data = Vec::with_capacity(rows.len());
    for (idx, row) in rows.into_iter().enumerate() {
        let gender: i8 = row.get("gender");
        let suffix = if gender == 0 { "_0" } else { "_1" };
        let hair_style = match row.get::<Option<String>, _>("hair_style") {
            Some(ref h) if !h.is_empty() => {
                if let Ok(num) = h.parse::<i64>() {
                    format!("hair_{num:02}{suffix}")
                } else {
                    h.clone()
                }
            }
            _ => format!("hair_01{suffix}"),
        };
        data.push(json!({
            "name": row.get::<String, _>("name"),
            "kill": row.get::<u32, _>("battles"),
            "rank": idx + 1,
            "sets": {
                "hair_style": hair_style,
                "hair_color": row.get::<Option<String>, _>("hair_color").unwrap_or_else(|| "0|0".into()),
                "skin_color": row.get::<Option<String>, _>("skin_color").unwrap_or_else(|| "0|0".into()),
                "weapon": row.get::<Option<String>, _>("equipment_weapon").unwrap_or_else(|| "wpn_01".into()),
                "back": row.get::<Option<String>, _>("equipment_back").unwrap_or_else(|| "back_01".into()),
                "clothing": row.get::<Option<String>, _>("equipment_clothing").unwrap_or_else(|| format!("set_01{suffix}")),
                "accessory": row.get::<Option<String>, _>("equipment_accessory").unwrap_or_else(|| "accessory_01".into()),
                "face": format!("face_01{suffix}"),
            }
        }));
    }

    Ok(json!({
        "status": 1,
        "error": 0,
        "title": "Kill Count",
        "data": data,
    }))
}

/// Dispatches `PvP.*` / `PvPService.*` AMF methods.
pub async fn dispatch_pvp(pool: &MySqlPool, method: &str, args: &[Json]) -> Result {
    match method {
        "checkAccess" => pvp_check_access(pool, args).await,
        "getCharacterStats" => pvp_get_character_stats(pool, args).await,
        "getLeaderboard" => pvp_get_leaderboard(pool, args).await,
        "getBattleActivity" => pvp_get_battle_activity(pool, args).await,
        "getDetailBattle" => pvp_get_detail_battle(pool, args).await,
        "reportBug" => pvp_report_bug(pool, args).await,
        _ => Ok(session_error("Invalid PvP method")),
    }
}

async fn pvp_check_access(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    Ok(json!({
        "status": 1,
        "result": "Access Granted",
        "maintenance": 0,
    }))
}

fn format_pvp_char_row(row: &sqlx::mysql::MySqlRow) -> Json {
    let id: u64 = row.get("id");
    let gender: i8 = row.get("gender");
    let suffix = if gender == 0 { "_0" } else { "_1" };
    let hair_style = match row.get::<Option<String>, _>("hair_style") {
        Some(ref h) if !h.is_empty() => {
            if let Ok(num) = h.parse::<i64>() {
                format!("hair_{num:02}{suffix}")
            } else {
                h.clone()
            }
        }
        _ => format!("hair_01{suffix}"),
    };
    json!({
        "id": id,
        "character_data": {
            "character_id": id,
            "character_name": row.get::<String, _>("name"),
            "character_level": row.get::<i32, _>("level"),
            "character_xp": row.get::<i64, _>("xp"),
            "character_gender": gender,
            "character_rank": row.get::<i8, _>("rank"),
            "character_merit": 0,
            "character_prestige": row.get::<i32, _>("prestige"),
            "character_element_1": row.get::<i8, _>("element_1"),
            "character_element_2": row.get::<i8, _>("element_2"),
            "character_element_3": row.get::<i8, _>("element_3"),
            "character_talent_1": row.get::<Option<String>, _>("talent_1"),
            "character_talent_2": row.get::<Option<String>, _>("talent_2"),
            "character_talent_3": row.get::<Option<String>, _>("talent_3"),
            "character_gold": row.get::<i32, _>("gold"),
            "character_tp": row.get::<i32, _>("tp"),
            "character_ss": row.get::<i32, _>("ss"),
            "character_class": row.get::<Option<String>, _>("class"),
            "character_senjutsu": row.get::<Option<String>, _>("senjutsu_type"),
            "character_pvp_points": row.get::<i32, _>("pvp_points"),
        },
        "character_points": {
            "atrrib_wind": row.get::<i32, _>("point_wind"),
            "atrrib_fire": row.get::<i32, _>("point_fire"),
            "atrrib_lightning": row.get::<i32, _>("point_lightning"),
            "atrrib_water": row.get::<i32, _>("point_water"),
            "atrrib_earth": row.get::<i32, _>("point_earth"),
            "atrrib_free": row.get::<i32, _>("point_free"),
        },
        "character_sets": {
            "weapon": row.get::<Option<String>, _>("equipment_weapon").unwrap_or_else(|| "wpn_01".into()),
            "back_item": row.get::<Option<String>, _>("equipment_back").unwrap_or_else(|| "back_01".into()),
            "clothing": row.get::<Option<String>, _>("equipment_clothing").unwrap_or_else(|| format!("set_01{suffix}")),
            "accessory": row.get::<Option<String>, _>("equipment_accessory").unwrap_or_else(|| "accessory_01".into()),
            "hairstyle": hair_style,
            "skills": row.get::<Option<String>, _>("equipment_skills").unwrap_or_default(),
            "senjutsu_skills": row.get::<Option<String>, _>("senjutsu_equipped_skills").unwrap_or_default(),
            "hair_color": row.get::<Option<String>, _>("hair_color").unwrap_or_else(|| "0|0".into()),
            "skin_color": row.get::<Option<String>, _>("skin_color").unwrap_or_else(|| "0|0".into()),
            "face": format!("face_01{suffix}"),
            "pet": row.get::<Option<String>, _>("equipment_pet").unwrap_or_default(),
        },
        "account_type": row.try_get::<i8, _>("account_type").unwrap_or(0),
    })
}

async fn pvp_battle_history_list(
    pool: &MySqlPool,
    char_id: i64,
    limit: i64,
) -> std::result::Result<Vec<Json>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT b.*, h.name AS host_name, e.name AS enemy_name \
         FROM pvp_battles b \
         LEFT JOIN characters h ON h.id = b.host_id \
         LEFT JOIN characters e ON e.id = b.enemy_id \
         WHERE b.host_id=? OR b.enemy_id=? \
         ORDER BY b.created_at DESC, b.id DESC LIMIT ?",
    )
    .bind(char_id)
    .bind(char_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let mut list = Vec::with_capacity(rows.len());
    for row in rows {
        let bid: u64 = row.get("id");
        let host_id = row.get::<u64, _>("host_id") as i64;
        let enemy_id = row.get::<u64, _>("enemy_id") as i64;
        let host_won: bool = row.get("host_won");
        let trophy_delta: i32 = row.get("trophy_delta");
        let is_host = host_id == char_id;
        let won = if is_host { host_won } else { !host_won };
        let signed_delta = if won {
            trophy_delta.abs()
        } else {
            -trophy_delta.abs()
        };
        let host_name = row
            .get::<Option<String>, _>("host_name")
            .unwrap_or_else(|| format!("#{host_id}"));
        let enemy_name = row
            .get::<Option<String>, _>("enemy_name")
            .unwrap_or_else(|| format!("#{enemy_id}"));
        let host_level: i32 = row.get("host_level");
        let enemy_level: i32 = row.get("enemy_level");
        let (char_str, enemy_str) = if is_host {
            (
                format!("{host_name} (Lv.{host_level})"),
                format!("{enemy_name} (Lv.{enemy_level})"),
            )
        } else {
            (
                format!("{enemy_name} (Lv.{enemy_level})"),
                format!("{host_name} (Lv.{host_level})"),
            )
        };
        list.push(json!({
            "id": bid,
            "character": char_str,
            "enemy": enemy_str,
            "won": won,
            "trophy": signed_delta,
            "date": "just now",
        }));
    }
    Ok(list)
}

async fn pvp_get_character_stats(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(row) = sqlx::query(
        "SELECT c.*, u.account_type FROM characters c JOIN users u ON u.id = c.user_id WHERE c.id=?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(session_error("Character not found"));
    };
    let histories = pvp_battle_history_list(pool, id, 50).await?;
    Ok(json!({
        "status": 1,
        "error": 0,
        "char": format_pvp_char_row(&row),
        "played": row.get::<i32, _>("pvp_played"),
        "won": row.get::<i32, _>("pvp_won"),
        "lost": row.get::<i32, _>("pvp_lost"),
        "trophy": row.get::<i32, _>("pvp_trophy"),
        "points": row.get::<i32, _>("pvp_points"),
        "histories": histories,
    }))
}

async fn pvp_get_leaderboard(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let filter = string(args, 2);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(own) = sqlx::query("SELECT pvp_points, pvp_trophy FROM characters WHERE id=?")
        .bind(id)
        .fetch_optional(pool)
        .await?
    else {
        return Ok(session_error("Character not found"));
    };
    let query = if filter == "prestige" {
        "SELECT c.*, u.account_type FROM characters c JOIN users u ON u.id = c.user_id \
         ORDER BY c.pvp_points DESC, c.pvp_trophy DESC LIMIT 100"
    } else {
        "SELECT c.*, u.account_type FROM characters c JOIN users u ON u.id = c.user_id \
         ORDER BY c.pvp_trophy DESC, c.pvp_won DESC LIMIT 100"
    };
    let rows = sqlx::query(query).fetch_all(pool).await?;
    let mut characters = Vec::with_capacity(rows.len());
    for row in rows {
        let mut entry = format_pvp_char_row(&row);
        entry["pvp_trophy"] = json!(row.get::<i32, _>("pvp_trophy"));
        entry["pvp_points"] = json!(row.get::<i32, _>("pvp_points"));
        entry["pvp_won"] = json!(row.get::<i32, _>("pvp_won"));
        entry["pvp_lost"] = json!(row.get::<i32, _>("pvp_lost"));
        entry["pvp_played"] = json!(row.get::<i32, _>("pvp_played"));
        characters.push(entry);
    }
    Ok(json!({
        "status": 1,
        "error": 0,
        "characters": characters,
        "points": own.get::<i32, _>("pvp_points"),
        "trophy": own.get::<i32, _>("pvp_trophy"),
    }))
}

async fn pvp_get_battle_activity(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let list = pvp_battle_history_list(pool, id, 100).await?;
    Ok(json!({
        "status": 1,
        "error": 0,
        " battles": list.clone(),
        "battles": list,
    }))
}

async fn pvp_get_detail_battle(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let battle_id = integer(args, 2);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(row) = sqlx::query("SELECT * FROM pvp_battles WHERE id=?")
        .bind(battle_id)
        .fetch_optional(pool)
        .await?
    else {
        return Ok(session_error("Battle not found"));
    };
    let host_id = row.get::<u64, _>("host_id") as i64;
    let enemy_id = row.get::<u64, _>("enemy_id") as i64;
    let host_snap = row
        .try_get::<Option<Json>, _>("host_snapshot")
        .ok()
        .flatten()
        .unwrap_or(Json::Null);
    let enemy_snap = row
        .try_get::<Option<Json>, _>("enemy_snapshot")
        .ok()
        .flatten()
        .unwrap_or(Json::Null);

    let host_char = if host_snap.is_object() {
        host_snap
    } else if let Some(c_row) = sqlx::query(
        "SELECT c.*, u.account_type FROM characters c JOIN users u ON u.id = c.user_id WHERE c.id=?",
    )
    .bind(host_id)
    .fetch_optional(pool)
    .await?
    {
        format_pvp_char_row(&c_row)
    } else {
        json!({"id": host_id})
    };

    let enemy_char = if enemy_snap.is_object() {
        enemy_snap
    } else if let Some(c_row) = sqlx::query(
        "SELECT c.*, u.account_type FROM characters c JOIN users u ON u.id = c.user_id WHERE c.id=?",
    )
    .bind(enemy_id)
    .fetch_optional(pool)
    .await?
    {
        format_pvp_char_row(&c_row)
    } else {
        json!({"id": enemy_id})
    };

    let battle_data = row
        .try_get::<Option<Json>, _>("battle_data")
        .ok()
        .flatten()
        .unwrap_or_else(|| json!([]));

    Ok(json!({
        "status": 1,
        "error": 0,
        "battle": {
            "id": row.get::<u64, _>("id"),
            "host_won": row.get::<bool, _>("host_won"),
            "trophy": row.get::<i32, _>("trophy_delta"),
            "host": {
                "char": host_char,
                "trophy": {
                    "before": row.get::<i32, _>("host_trophy_before"),
                    "after": row.get::<i32, _>("host_trophy_after"),
                }
            },
            "enemy": {
                "char": enemy_char,
                "trophy": {
                    "before": row.get::<i32, _>("enemy_trophy_before"),
                    "after": row.get::<i32, _>("enemy_trophy_after"),
                }
            },
            "detail": battle_data,
        }
    }))
}

async fn pvp_report_bug(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let title = string(args, 2);
    let description = string(args, 3);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    if description.trim().is_empty() {
        return Ok(json!({
            "status": 2,
            "result": "Please provide a bug description."
        }));
    }
    let clean_title: String = if title.trim().is_empty() {
        "Bug Report".into()
    } else {
        title.trim().chars().take(255).collect()
    };
    let clean_desc: String = description.trim().chars().take(2000).collect();

    sqlx::query(
        "INSERT INTO pvp_bug_reports (character_id, title, description, created_at, updated_at) \
         VALUES (?, ?, ?, NOW(), NOW())",
    )
    .bind(id)
    .bind(clean_title)
    .bind(clean_desc)
    .execute(pool)
    .await?;

    Ok(json!({
        "status": 1,
        "error": 0,
        "result": "Thank you! Your bug report has been submitted.",
    }))
}

/// The stored `panel` entries are strings; absence keeps PHP's `?? 1` default.
fn icon(row: &sqlx::mysql::MySqlRow) -> Json {
    row.get::<Option<String>, _>("icon")
        .filter(|v| !v.is_empty())
        .map(Json::from)
        .unwrap_or(json!(1))
}

/// Accepts either positional `[id, key, ...]` arguments or a single wrapped
/// `[id, key]` array/object parameter, matching PHP's `$params[0], $params[1]`.
pub(crate) fn guard_pair(args: &[Json]) -> Option<(i64, String)> {
    for outer in args {
        let id = pick(outer, 0);
        let key = pick(outer, 1);
        if let (Some(id), Some(key)) = (id, key) {
            if id.is_null() || key.is_null() {
                continue;
            }
            return Some((
                integer(std::slice::from_ref(id), 0),
                string(std::slice::from_ref(key), 0),
            ));
        }
    }
    if args.len() >= 2 && !args[0].is_null() && !args[1].is_null() {
        return Some((integer(args, 0), string(args, 1)));
    }
    None
}

fn pick(value: &Json, index: usize) -> Option<&Json> {
    match value {
        Json::Array(list) => list.get(index),
        Json::Object(map) => map.get(&index.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_pair_supports_positional_and_wrapped_arguments() {
        let id = json!(5);
        let key = json!("test");
        assert_eq!(
            guard_pair(&[id.clone(), key.clone()]),
            Some((5, "test".into()))
        );
        assert_eq!(guard_pair(&[json!([5, "test"])]), Some((5, "test".into())));
        assert_eq!(
            guard_pair(&[json!({"0": 5, "1": "test"})]),
            Some((5, "test".into()))
        );
        assert_eq!(guard_pair(&[json!(null)]), None);
        assert_eq!(guard_pair(&[json!([5])]), None);
        assert_eq!(
            guard_pair(&[json!({"charId": 5, "sessionKey": "test"})]),
            None
        );
    }
}
