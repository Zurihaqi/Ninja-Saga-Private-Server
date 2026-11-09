//! Seasonal event AMF services (`MonsterHunterEvent2023`, `DragonHuntEvent`,
//! `JusticeBadgeEvent2024`, `HalloweenEvent2025`, `ConfrontingDeathEvent2025`,
//! `ThanksGivingEvent2025`, `ChristmasEvent2021`, `ChristmasEvent2025`, `NewYear2026`).
//!
//! Source:
//! - `app/Services/Amf/MonsterHunterEvent2023Service.php`
//! - `app/Services/Amf/DragonHuntEventService.php`
//! - `app/Services/Amf/JusticeBadgeEvent2024Service.php`
//! - `app/Services/Amf/HalloweenEvent2025Service.php`
//! - `app/Services/Amf/ConfrontingDeathEvent2025Service.php`
//! - `app/Services/Amf/ThanksGivingEvent2025Service.php`
//! - `app/Services/Amf/ChristmasEvent2021Service.php`
//! - `app/Services/Amf/ChristmasEvent2025Service.php`
//! - `app/Services/Amf/NewYear2026Service.php`
use crate::{
    GameState,
    account::{guard, integer, session_error, string},
    rewards,
};
use authentication as auth;
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};
use std::time::{SystemTime, UNIX_EPOCH};

type Result = std::result::Result<Json, sqlx::Error>;

#[derive(Clone, Debug)]
pub(crate) struct EventBattle {
    pub code: String,
    pub boss_id: String,
    pub started: std::time::Instant,
    pub rewards_cfg: Json,
}

#[derive(Clone, Debug)]
struct EventRow {
    id: u64,
    energy: i64,
    battles: i64,
    milestones_claimed: Json,
    bought: bool,
    extra: Json,
}

fn rand_u64() -> u64 {
    let hex = auth::random_hex().unwrap_or_else(|_| "0123456789abcdef".into());
    u64::from_str_radix(&hex[..16], 16).unwrap_or(0)
}

fn generate_code(len: usize) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let hex = auth::random_hex().unwrap_or_else(|_| "0123456789abcdef0123456789abcdef".into());
    hex.as_bytes()
        .chunks(2)
        .take(len)
        .map(|pair| {
            let b = u8::from_str_radix(std::str::from_utf8(pair).unwrap_or("00"), 16).unwrap_or(0);
            CHARS[(b as usize) % CHARS.len()] as char
        })
        .collect()
}

fn now_hm() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    format!("{h:02}:{m:02}")
}

/// Supports both `(charId, sessionKey, ...)` and `(sessionKey, charId, ...)` parameter orders.
fn extract_id_key(args: &[Json]) -> (i64, String) {
    let a0_int = integer(args, 0);
    let a1_int = integer(args, 1);
    let a0_str = string(args, 0);
    let a1_str = string(args, 1);
    if a0_int > 0 && (a1_int == 0 || a1_str.len() >= 8) {
        (a0_int, a1_str)
    } else if a1_int > 0 {
        (a1_int, a0_str)
    } else {
        (a0_int, a1_str)
    }
}

async fn active_event_config(
    pool: &MySqlPool,
    panel: &str,
) -> std::result::Result<Option<Json>, sqlx::Error> {
    let row =
        sqlx::query("SELECT data FROM game_events WHERE panel=? AND active=1 ORDER BY id LIMIT 1")
            .bind(panel)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|r| {
        let mut cfg = r
            .try_get::<Option<Json>, _>("data")
            .ok()
            .flatten()
            .unwrap_or_else(|| json!({}));
        if cfg.get("milestones").is_none() && cfg.get("milestone_battle").is_some() {
            cfg["milestones"] = cfg["milestone_battle"].clone();
        }
        if cfg.get("gacha_milestones").is_none() && cfg["gacha"].get("milestone").is_some() {
            cfg["gacha_milestones"] = cfg["gacha"]["milestone"].clone();
        }
        cfg
    }))
}

async fn get_or_create_event_row(
    pool: &MySqlPool,
    char_id: i64,
    event_key: &str,
    energy_max: i64,
) -> std::result::Result<EventRow, sqlx::Error> {
    if let Some(row) = sqlx::query(
        "SELECT id, energy, battles, milestones_claimed, bought, extra \
         FROM character_event_data WHERE character_id=? AND event_key=?",
    )
    .bind(char_id)
    .bind(event_key)
    .fetch_optional(pool)
    .await?
    {
        return Ok(parse_event_row(&row));
    }
    sqlx::query(
        "INSERT IGNORE INTO character_event_data \
         (character_id, event_key, energy, battles, milestones_claimed, bought, extra, created_at, updated_at) \
         VALUES (?, ?, ?, 0, '[]', 0, '{}', NOW(), NOW())",
    )
    .bind(char_id)
    .bind(event_key)
    .bind(energy_max.max(0))
    .execute(pool)
    .await?;

    let row = sqlx::query(
        "SELECT id, energy, battles, milestones_claimed, bought, extra \
         FROM character_event_data WHERE character_id=? AND event_key=?",
    )
    .bind(char_id)
    .bind(event_key)
    .fetch_one(pool)
    .await?;
    Ok(parse_event_row(&row))
}

fn parse_event_row(row: &sqlx::mysql::MySqlRow) -> EventRow {
    EventRow {
        id: row.get("id"),
        energy: i64::from(row.get::<u16, _>("energy")),
        battles: i64::from(row.get::<u32, _>("battles")),
        milestones_claimed: row
            .try_get::<Option<Json>, _>("milestones_claimed")
            .ok()
            .flatten()
            .unwrap_or_else(|| json!([])),
        bought: row.get("bought"),
        extra: row
            .try_get::<Option<Json>, _>("extra")
            .ok()
            .flatten()
            .unwrap_or_else(|| json!({})),
    }
}

fn resolve_gender_placeholder(id_str: &str, gender: i8) -> String {
    let base = id_str.split(':').next().unwrap_or(id_str);
    let suffix = if gender == 0 { "_0" } else { "_1" };
    base.replace("%s", suffix)
}

async fn award_event_battle(
    pool: &MySqlPool,
    char_id: i64,
    event_row_id: u64,
    win_flag: i64,
    rewards_cfg: &Json,
) -> std::result::Result<(i64, i64, Vec<String>, i64, i64, bool), sqlx::Error> {
    let mut tx = pool.begin().await?;
    let Some(c_row) = sqlx::query("SELECT level, xp, gender FROM characters WHERE id=? FOR UPDATE")
        .bind(char_id)
        .fetch_optional(&mut *tx)
        .await?
    else {
        return Ok((0, 0, Vec::new(), 1, 0, false));
    };
    let mut level = i64::from(c_row.get::<i32, _>("level"));
    let mut xp = c_row.get::<i64, _>("xp");
    let gender: i8 = c_row.get("gender");

    if win_flag != 1 {
        tx.commit().await?;
        return Ok((0, 0, Vec::new(), level, xp, false));
    }

    sqlx::query(
        "UPDATE character_event_data SET battles = battles + 1, updated_at=NOW() WHERE id=?",
    )
    .bind(event_row_id)
    .execute(&mut *tx)
    .await?;

    let xp_base = boss_amount(&rewards_cfg["xp"], level);
    let xp_mul = rewards_cfg
        .get("xp_level_multiplier")
        .and_then(Json::as_i64)
        .unwrap_or(0);
    let gold_base = boss_amount(&rewards_cfg["gold"], level);
    let gold_mul = rewards_cfg
        .get("gold_level_multiplier")
        .and_then(Json::as_i64)
        .unwrap_or(0);

    let calc_xp = if xp_mul > 0 {
        level * xp_mul / 600
    } else {
        xp_base
    };
    let gold_awarded = if gold_mul > 0 {
        level * gold_mul / 600
    } else {
        gold_base
    };

    rewards::add_gold(&mut tx, char_id, gold_awarded).await?;
    let (new_lvl, new_xp, mut level_up) = rewards::add_xp(&mut tx, char_id, calc_xp).await?;
    let xp_awarded = calc_xp;
    level = i64::from(new_lvl);
    xp = new_xp;

    let mut items_awarded = Vec::new();
    let guaranteed = rewards_cfg
        .get("guaranteed")
        .and_then(Json::as_array)
        .or_else(|| rewards_cfg.get("items").and_then(Json::as_array))
        .or_else(|| rewards_cfg.get("rewards").and_then(Json::as_array))
        .or_else(|| rewards_cfg.as_array());
    if let Some(arr) = guaranteed {
        for entry in arr {
            let (raw_id, chance) = if let Some(s) = entry.as_str() {
                (s, 100i64)
            } else if let Some(obj) = entry.as_object() {
                (
                    obj.get("id").and_then(Json::as_str).unwrap_or(""),
                    obj.get("chance").and_then(Json::as_i64).unwrap_or(100),
                )
            } else {
                ("", 0)
            };
            if !raw_id.is_empty() && ((rand_u64() % 100) as i64) < chance {
                if rewards::grant(&mut tx, char_id, raw_id).await? {
                    level_up = true;
                }
                items_awarded.push(resolve_gender_placeholder(raw_id, gender));
            }
        }
    }
    if let Some(drops) = rewards_cfg.get("drops").and_then(Json::as_array) {
        for entry in drops {
            if let Some(obj) = entry.as_object() {
                let raw_id = obj.get("id").and_then(Json::as_str).unwrap_or("");
                let chance = obj.get("chance").and_then(Json::as_i64).unwrap_or(100);
                if !raw_id.is_empty() && ((rand_u64() % 100) as i64) < chance {
                    if rewards::grant(&mut tx, char_id, raw_id).await? {
                        level_up = true;
                    }
                    items_awarded.push(resolve_gender_placeholder(raw_id, gender));
                }
            }
        }
    }

    if level_up {
        let Some(refreshed) = sqlx::query("SELECT level, xp FROM characters WHERE id=?")
            .bind(char_id)
            .fetch_optional(&mut *tx)
            .await?
        else {
            tx.commit().await?;
            return Ok((xp_awarded, gold_awarded, items_awarded, level, xp, level_up));
        };
        level = i64::from(refreshed.get::<i32, _>("level"));
        xp = refreshed.get::<i64, _>("xp");
    }

    tx.commit().await?;
    Ok((xp_awarded, gold_awarded, items_awarded, level, xp, level_up))
}

fn weighted_pick(pool: &[Json]) -> String {
    let total_weight: i64 = pool
        .iter()
        .map(|e| e.get("weight").and_then(Json::as_i64).unwrap_or(1).max(1))
        .sum();
    if total_weight <= 0 || pool.is_empty() {
        return "gold_10000".into();
    }
    let mut roll = ((rand_u64() % (total_weight as u64)) as i64) + 1;
    for entry in pool {
        let w = entry
            .get("weight")
            .and_then(Json::as_i64)
            .unwrap_or(1)
            .max(1);
        roll -= w;
        if roll <= 0 {
            return entry
                .get("id")
                .and_then(Json::as_str)
                .unwrap_or("gold_10000")
                .to_string();
        }
    }
    pool.last()
        .and_then(|e| e.get("id"))
        .and_then(Json::as_str)
        .unwrap_or("gold_10000")
        .to_string()
}

// ---------------------------------------------------------------------------
// 1. MonsterHunterEvent2023
// ---------------------------------------------------------------------------

pub async fn dispatch_monster_hunter(
    state: &GameState,
    pool: &MySqlPool,
    method: &str,
    args: &[Json],
) -> Result {
    match method {
        "getEventData" => mh_get_event_data(pool, args).await,
        "startBattle" => mh_start_battle(state, pool, args).await,
        "finishBattle" => mh_finish_battle(state, pool, args).await,
        _ => Ok(session_error("Invalid MonsterHunterEvent2023 method")),
    }
}

async fn mh_get_event_data(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(cfg) = active_event_config(pool, "MonsterHunter").await? else {
        return Ok(json!({"status": 2, "result": "Monster Hunter event is not currently active."}));
    };
    let energy_max = cfg.get("energy_max").and_then(Json::as_i64).unwrap_or(100);
    let boss_id = cfg
        .get("boss_id")
        .and_then(Json::as_str)
        .unwrap_or("ene_2123");
    let row = get_or_create_event_row(pool, id, "monster_hunter", energy_max).await?;
    Ok(json!({
        "status": 1,
        "error": 0,
        "energy": row.energy,
        "boss_id": boss_id,
    }))
}

async fn mh_start_battle(state: &GameState, pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let boss_id = string(args, 1);
    let key = string(args, 3);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    if !auth::valid_legacy_hash(&string(args, 2), &format!("{id}{boss_id}")) {
        return Ok(json!({"status":2,"result":"Security check failed."}));
    }
    let Some(cfg) = active_event_config(pool, "MonsterHunter").await? else {
        return Ok(json!({"status": 2, "result": "Monster Hunter event is not currently active."}));
    };
    let energy_max = cfg.get("energy_max").and_then(Json::as_i64).unwrap_or(100);
    let cost = cfg.get("energy_cost").and_then(Json::as_i64).unwrap_or(20);
    let row = get_or_create_event_row(pool, id, "monster_hunter", energy_max).await?;
    if row.energy < cost {
        return Ok(json!({"status": 2, "result": "Not enough energy to hunt."}));
    }
    sqlx::query("UPDATE character_event_data SET energy = energy - ?, updated_at=NOW() WHERE id=?")
        .bind(cost)
        .bind(row.id)
        .execute(pool)
        .await?;

    let code = generate_code(10);
    let hash = auth::sha256_hex(&format!("{boss_id}{code}{id}"));
    let rewards_cfg = cfg.get("rewards").cloned().unwrap_or_else(|| json!({}));
    state.event_battles.lock().await.insert(
        (id, "monster_hunter".into()),
        EventBattle {
            code: code.clone(),
            boss_id: boss_id.clone(),
            started: std::time::Instant::now(),
            rewards_cfg,
        },
    );
    Ok(json!({
        "status": 1,
        "error": 0,
        "code": code,
        "hash": hash,
    }))
}

async fn mh_finish_battle(state: &GameState, pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let code = string(args, 2);
    let win_flag = integer(args, 5);
    let key = string(args, 6);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let mut battles = state.event_battles.lock().await;
    let map_key = (id, "monster_hunter".into());
    let Some(battle) = battles.get(&map_key) else {
        return Ok(json!({"status":2,"result":"Invalid or expired battle session."}));
    };
    if battle.code != code
        || battle.boss_id != string(args, 1)
        || battle.started.elapsed().as_secs() >= 1800
    {
        return Ok(json!({"status":2,"result":"Invalid or expired battle session."}));
    }
    let expected = format!(
        "{id}{}{}{}{}",
        string(args, 1),
        code,
        string(args, 3),
        string(args, 5)
    );
    if !auth::valid_legacy_hash(&string(args, 4), &expected) {
        return Ok(json!({"status":2,"result":"Security check failed."}));
    }
    let row = get_or_create_event_row(pool, id, "monster_hunter", 100).await?;
    let (xp_awarded, gold_awarded, items_awarded, level, xp, level_up) =
        award_event_battle(pool, id, row.id, win_flag, &battle.rewards_cfg).await?;
    battles.remove(&map_key);
    Ok(json!({
        "status": 1,
        "error": 0,
        "result": [xp_awarded,gold_awarded,items_awarded],
        "level": level,
        "xp": xp,
        "level_up": level_up,
        "energy": row.energy,
    }))
}

// ---------------------------------------------------------------------------
// 2. DragonHuntEvent
// ---------------------------------------------------------------------------

pub async fn dispatch_dragon_hunt(
    state: &GameState,
    pool: &MySqlPool,
    method: &str,
    args: &[Json],
) -> Result {
    match method {
        "startBattle" => dh_start_battle(state, pool, args).await,
        "finishBattle" => dh_finish_battle(state, pool, args).await,
        "buyMaterial" => dh_buy_material(pool, args).await,
        "getGachaData" => dh_get_gacha_data(pool, args).await,
        "getGachaRewards" => dh_get_gacha_rewards(state, pool, args).await,
        "getGlobalGachaHistory" => dh_get_global_history(state, pool, args).await,
        _ => Ok(session_error("Invalid DragonHuntEvent method")),
    }
}

async fn dh_config(pool: &MySqlPool) -> std::result::Result<Option<Json>, sqlx::Error> {
    if let Some(cfg) = active_event_config(pool, "DragonHunt").await? {
        return Ok(Some(cfg));
    }
    active_event_config(pool, "DragonGacha").await
}

async fn dh_start_battle(state: &GameState, pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let boss_id = string(args, 1);
    let mode = integer(args, 2);
    let key = string(args, 6);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    if !(0..=2).contains(&mode)
        || !auth::valid_legacy_hash(
            &string(args, 5),
            &format!("{id}{boss_id}{mode}{}{}", string(args, 4), string(args, 3)),
        )
    {
        return Ok(json!({"status":2,"result":"Security check failed."}));
    }
    let Some(cfg) = active_event_config(pool, "DragonHunt").await? else {
        return Ok(json!({"status": 2, "result": "Dragon Hunt event is not currently active."}));
    };
    let token_cost = if mode == 2 {
        cfg["easy_mode_token_cost"].as_i64().unwrap_or(100)
    } else {
        0
    };
    let gold_cost = if mode == 1 {
        cfg["normal_mode_gold_cost"].as_i64().unwrap_or(250000)
    } else {
        0
    };
    if token_cost < 0 || gold_cost < 0 {
        return Ok(json!({"status":2,"result":"Invalid Dragon Hunt price."}));
    }
    if token_cost > 0 || gold_cost > 0 {
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
        if token_cost > 0 {
            let tokens = i64::from(
                sqlx::query_scalar::<_, i32>("SELECT tokens FROM users WHERE id=? FOR UPDATE")
                    .bind(user_id)
                    .fetch_one(&mut *tx)
                    .await?,
            );
            if tokens < token_cost {
                return Ok(json!({"status": 2, "result": "Not enough tokens for Hard mode."}));
            }
            sqlx::query("UPDATE users SET tokens=?, updated_at=NOW() WHERE id=?")
                .bind(tokens - token_cost)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
        }
        if gold_cost > 0 {
            if gold < gold_cost {
                return Ok(json!({"status": 2, "result": "Not enough gold."}));
            }
            sqlx::query("UPDATE characters SET gold=?, updated_at=NOW() WHERE id=?")
                .bind(gold - gold_cost)
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
    }

    let _ = get_or_create_event_row(pool, id, "dragon_hunt", 0).await?;
    let ranges = cfg["capture_range"]
        .get(mode.to_string())
        .and_then(Json::as_array);
    let n1 = ranges
        .and_then(|a| a.first())
        .and_then(Json::as_i64)
        .unwrap_or(0);
    let n2 = ranges
        .and_then(|a| a.get(1))
        .and_then(Json::as_i64)
        .unwrap_or([5, 15, 25][mode as usize]);
    let code = generate_code(16);
    let hash = auth::sha256_hex(&format!("{boss_id}{code}{id}{n1}{n2}"));
    let rewards_cfg = cfg["rewards_per_boss"]
        .get(&boss_id)
        .cloned()
        .unwrap_or_else(|| {
            cfg.get("rewards_default")
                .cloned()
                .unwrap_or_else(|| json!([]))
        });
    state.event_battles.lock().await.insert(
        (id, "dragon_hunt".into()),
        EventBattle {
            code: code.clone(),
            boss_id: boss_id.clone(),
            started: std::time::Instant::now(),
            rewards_cfg,
        },
    );
    Ok(json!({
        "status": 1,
        "error": 0,
        "code": code,
        "hash": hash,
        "n1":n1,"n2":n2,
    }))
}

async fn dh_finish_battle(state: &GameState, pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let code = string(args, 2);
    let win_flag = i64::from(integer(args, 8) >= 0);
    let key = string(args, 5);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let mut battles = state.event_battles.lock().await;
    let map_key = (id, "dragon_hunt".into());
    let Some(battle) = battles.get(&map_key) else {
        return Ok(json!({"status":2,"result":"Invalid or expired battle session."}));
    };
    if battle.code != code
        || battle.boss_id != string(args, 1)
        || battle.started.elapsed().as_secs() >= 1800
    {
        return Ok(json!({"status":2,"result":"Invalid or expired battle session."}));
    }
    let expected = format!(
        "{id}{}{}{}{}{}{}",
        string(args, 1),
        code,
        string(args, 4),
        string(args, 6),
        string(args, 7),
        string(args, 8)
    );
    if !auth::valid_legacy_hash(&string(args, 3), &expected) {
        return Ok(json!({"status":2,"result":"Security check failed."}));
    }
    let row = get_or_create_event_row(pool, id, "dragon_hunt", 0).await?;
    let (xp_awarded, gold_awarded, items_awarded, level, xp, level_up) =
        award_event_battle(pool, id, row.id, win_flag, &battle.rewards_cfg).await?;
    battles.remove(&map_key);
    Ok(json!({
        "status": 1,
        "error": 0,
        "result": [xp_awarded,gold_awarded,items_awarded],
        "level": level,
        "xp": xp,
        "level_up": level_up,
    }))
}

async fn dh_buy_material(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let item_id = string(args, 2);
    let qty = integer(args, 3);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    if qty <= 0 || item_id.is_empty() {
        return Ok(json!({"status": 2, "result": "Invalid purchase parameters."}));
    }
    let Some(cfg) = dh_config(pool).await? else {
        return Ok(json!({"status": 2, "result": "Dragon Hunt event is not currently active."}));
    };
    let price_per_unit = cfg
        .get("material_price")
        .and_then(|v| {
            v.as_i64()
                .or_else(|| v.get(&item_id).and_then(Json::as_i64))
        })
        .unwrap_or(50);
    let total_cost = price_per_unit * qty;

    let mut tx = pool.begin().await?;
    let Some(user_id) =
        sqlx::query_scalar::<_, u64>("SELECT user_id FROM characters WHERE id=? FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
    else {
        return Ok(session_error("Character not found"));
    };
    let tokens = i64::from(
        sqlx::query_scalar::<_, i32>("SELECT tokens FROM users WHERE id=? FOR UPDATE")
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?,
    );
    if tokens < total_cost {
        return Ok(json!({"status": 2, "result": "Not enough tokens."}));
    }
    sqlx::query("UPDATE users SET tokens=?, updated_at=NOW() WHERE id=?")
        .bind(tokens - total_cost)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    rewards::give_item(&mut tx, id, &item_id, qty).await?;
    tx.commit().await?;
    Ok(json!({"status": 1, "error": 0}))
}

async fn dh_get_gacha_data(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(cfg) = dh_config(pool).await? else {
        return Ok(json!({"status": 2, "result": "Dragon Gacha is not currently active."}));
    };
    let gacha = cfg.get("gacha").unwrap_or(&cfg);
    let coin_cost = gacha
        .get("coin_cost")
        .cloned()
        .unwrap_or_else(|| json!([1, 2, 10]));
    let token_cost = gacha
        .get("token_cost")
        .cloned()
        .unwrap_or_else(|| json!([25, 50, 250]));
    Ok(json!({
        "status": 1,
        "error": 0,
        "coin": coin_cost,
        "token": token_cost,
        "histories": [],
    }))
}

async fn dh_get_gacha_rewards(state: &GameState, pool: &MySqlPool, args: &[Json]) -> Result {
    let (id, key) = extract_id_key(args);
    let play_type = string(args, 2);
    let play_qty = integer(args, 3).clamp(1, 10);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(cfg) = dh_config(pool).await? else {
        return Ok(json!({"status": 2, "result": "Dragon Gacha is not currently active."}));
    };
    let gacha = cfg.get("gacha").unwrap_or(&cfg);
    let tier_idx = if play_qty >= 10 {
        2
    } else if play_qty >= 2 {
        1
    } else {
        0
    };
    let pool_items = gacha
        .get("pool")
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!({"id": "gold_10000", "weight": 100})]);
    let coin_id = gacha
        .get("coin_id")
        .and_then(Json::as_str)
        .unwrap_or("material_778");

    let mut tx = pool.begin().await?;
    let Some(c_row) =
        sqlx::query("SELECT user_id, name, level, gender FROM characters WHERE id=? FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
    else {
        return Ok(session_error("Character not found"));
    };
    let user_id: u64 = c_row.get("user_id");
    let char_name: String = c_row.get("name");
    let char_level: i32 = c_row.get("level");
    let gender: i8 = c_row.get("gender");

    if play_type == "coins" {
        let coin_costs = gacha
            .get("coin_cost")
            .and_then(Json::as_array)
            .cloned()
            .unwrap_or_else(|| vec![json!(1), json!(2), json!(10)]);
        let needed = coin_costs
            .get(tier_idx)
            .and_then(Json::as_i64)
            .unwrap_or(play_qty);
        let item_row = sqlx::query(
            "SELECT id, quantity FROM character_items WHERE character_id=? AND item_id=? FOR UPDATE",
        )
        .bind(id)
        .bind(coin_id)
        .fetch_optional(&mut *tx)
        .await?;
        let (row_id, have) = match item_row {
            Some(r) => (
                r.get::<u64, _>("id"),
                i64::from(r.get::<i32, _>("quantity")),
            ),
            None => (0, 0),
        };
        if have < needed {
            return Ok(json!({"status": 2, "result": "Not enough gacha coins."}));
        }
        if have == needed {
            sqlx::query("DELETE FROM character_items WHERE id=?")
                .bind(row_id)
                .execute(&mut *tx)
                .await?;
        } else {
            sqlx::query("UPDATE character_items SET quantity=?, updated_at=NOW() WHERE id=?")
                .bind(have - needed)
                .bind(row_id)
                .execute(&mut *tx)
                .await?;
        }
    } else {
        let token_costs = gacha
            .get("token_cost")
            .and_then(Json::as_array)
            .cloned()
            .unwrap_or_else(|| vec![json!(25), json!(50), json!(250)]);
        let needed = token_costs
            .get(tier_idx)
            .and_then(Json::as_i64)
            .unwrap_or(25 * play_qty);
        let tokens = i64::from(
            sqlx::query_scalar::<_, i32>("SELECT tokens FROM users WHERE id=? FOR UPDATE")
                .bind(user_id)
                .fetch_one(&mut *tx)
                .await?,
        );
        if tokens < needed {
            return Ok(json!({"status": 2, "result": "Not enough tokens."}));
        }
        sqlx::query("UPDATE users SET tokens=?, updated_at=NOW() WHERE id=?")
            .bind(tokens - needed)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
    }

    let mut granted = Vec::with_capacity(play_qty as usize);
    for _ in 0..play_qty {
        let picked = weighted_pick(&pool_items);
        rewards::grant(&mut tx, id, &picked).await?;
        granted.push(resolve_gender_placeholder(&picked, gender));
    }
    tx.commit().await?;

    let row = get_or_create_event_row(pool, id, "dragon_hunt", 0).await?;
    sqlx::query(
        "UPDATE character_event_data SET battles = battles + ?, updated_at=NOW() WHERE id=?",
    )
    .bind(play_qty)
    .bind(row.id)
    .execute(pool)
    .await?;

    let obtained_at = now_hm();
    let mut g_map = state.gacha_histories.lock().await;
    let hist = g_map.entry("dragon_hunt".into()).or_default();
    for r_id in &granted {
        hist.insert(
            0,
            json!({
                "id": id,
                "name": char_name,
                "level": char_level,
                "reward": r_id,
                "spin": play_qty,
                "obtained_at": obtained_at,
            }),
        );
    }
    hist.truncate(100);

    Ok(json!({
        "status": 1,
        "error": 0,
        "rewards": granted,
    }))
}

async fn dh_get_global_history(state: &GameState, pool: &MySqlPool, args: &[Json]) -> Result {
    let (id, key) = extract_id_key(args);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let histories = state
        .gacha_histories
        .lock()
        .await
        .get("dragon_hunt")
        .cloned()
        .unwrap_or_default();
    Ok(json!({
        "status": 1,
        "error": 0,
        "histories": histories,
    }))
}

// ---------------------------------------------------------------------------
// 3. JusticeBadgeEvent2024
// ---------------------------------------------------------------------------

pub async fn dispatch_justice_badge(pool: &MySqlPool, method: &str, args: &[Json]) -> Result {
    match method {
        "getEventData" => jb_get_event_data(pool, args).await,
        "exchange" => jb_exchange(pool, args).await,
        _ => Ok(session_error("Invalid JusticeBadgeEvent2024 method")),
    }
}

async fn jb_get_event_data(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    if active_event_config(pool, "JusticeBadge").await?.is_none() {
        return Ok(json!({"status": 2, "result": "Justice Badge event is not currently active."}));
    }
    let row = get_or_create_event_row(pool, id, "justice_badge_2024", 0).await?;
    Ok(json!({
        "status": 1,
        "error": 0,
        "claimed": row.milestones_claimed,
    }))
}

async fn jb_exchange(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let reward_idx = integer(args, 2) as usize;
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(cfg) = active_event_config(pool, "JusticeBadge").await? else {
        return Ok(json!({"status": 2, "result": "Justice Badge event is not currently active."}));
    };
    let rewards_list = cfg
        .get("rewards")
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_default();
    let costs_list = cfg
        .get("costs")
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_default();
    let material_id = cfg
        .get("material_id")
        .and_then(Json::as_str)
        .unwrap_or("material_2110");

    let Some(reward_entry) = rewards_list.get(reward_idx) else {
        return Ok(json!({"status": 2, "result": "Invalid reward index."}));
    };
    let raw_id = reward_entry
        .as_str()
        .or_else(|| reward_entry.get("id").and_then(Json::as_str))
        .unwrap_or("");
    let cost = reward_entry
        .get("cost")
        .and_then(Json::as_i64)
        .or_else(|| costs_list.get(reward_idx).and_then(Json::as_i64))
        .unwrap_or(0);
    if raw_id.is_empty() {
        return Ok(json!({"status": 2, "result": "Reward configuration error."}));
    }

    let row = get_or_create_event_row(pool, id, "justice_badge_2024", 0).await?;
    let mut claimed = row
        .milestones_claimed
        .as_array()
        .cloned()
        .unwrap_or_default();
    if claimed
        .iter()
        .any(|v| v.as_u64() == Some(reward_idx as u64))
    {
        return Ok(json!({"status": 2, "result": "You have already claimed this reward."}));
    }

    let mut tx = pool.begin().await?;
    let gender: i8 = sqlx::query_scalar("SELECT gender FROM characters WHERE id=? FOR UPDATE")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    if cost > 0 {
        let m_row = sqlx::query(
            "SELECT id, quantity FROM character_items WHERE character_id=? AND item_id=? FOR UPDATE",
        )
        .bind(id)
        .bind(material_id)
        .fetch_optional(&mut *tx)
        .await?;
        let (m_id, have) = match m_row {
            Some(r) => (
                r.get::<u64, _>("id"),
                i64::from(r.get::<i32, _>("quantity")),
            ),
            None => (0, 0),
        };
        if have < cost {
            return Ok(json!({"status": 2, "result": "Not enough Justice Badges."}));
        }
        if have == cost {
            sqlx::query("DELETE FROM character_items WHERE id=?")
                .bind(m_id)
                .execute(&mut *tx)
                .await?;
        } else {
            sqlx::query("UPDATE character_items SET quantity=?, updated_at=NOW() WHERE id=?")
                .bind(have - cost)
                .bind(m_id)
                .execute(&mut *tx)
                .await?;
        }
    }
    rewards::grant(&mut tx, id, raw_id).await?;
    claimed.push(json!(reward_idx));
    sqlx::query(
        "UPDATE character_event_data SET milestones_claimed=?, updated_at=NOW() WHERE id=?",
    )
    .bind(serde_json::to_string(&claimed).unwrap_or_else(|_| "[]".into()))
    .bind(row.id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(json!({
        "status": 1,
        "error": 0,
        "reward": resolve_gender_placeholder(raw_id, gender),
    }))
}

// ---------------------------------------------------------------------------
// 4. Shared 2025 Menu Event Helpers (Halloween, ConfrontingDeath, ThanksGiving, Christmas2025)
// ---------------------------------------------------------------------------

async fn menu_get_battle_data(
    pool: &MySqlPool,
    args: &[Json],
    panel: &str,
    event_key: &str,
    default_energy_max: i64,
) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(cfg) = active_event_config(pool, panel).await? else {
        return Ok(json!({"status": 2, "result": "Event is not currently active."}));
    };
    let energy_max = cfg
        .get("energy_max")
        .and_then(Json::as_i64)
        .unwrap_or(default_energy_max);
    let row = get_or_create_event_row(pool, id, event_key, energy_max).await?;
    Ok(json!({
        "status": 1,
        "error": 0,
        "energy": row.energy,
    }))
}

fn find_boss<'a>(value: &'a Json, id: &str) -> Option<&'a Json> {
    if value.as_str() == Some(id) {
        return Some(value);
    }
    if let Some(obj) = value.as_object() {
        if obj.get("id").is_some_and(|ids| contains_boss_id(ids, id)) {
            return Some(value);
        }
        return obj.values().find_map(|v| find_boss(v, id));
    }
    value.as_array()?.iter().find_map(|v| find_boss(v, id))
}
fn contains_boss_id(value: &Json, id: &str) -> bool {
    if value.as_str() == Some(id) {
        return true;
    }
    if let Some(a) = value.as_array() {
        return a.iter().any(|v| contains_boss_id(v, id));
    }
    value
        .as_object()
        .is_some_and(|m| m.values().any(|v| contains_boss_id(v, id)))
}
fn boss_amount(value: &Json, level: i64) -> i64 {
    if let Some(n) = value.as_i64() {
        return n.max(0);
    }
    let Some(formula) = value.as_str() else {
        return 0;
    };
    // Only the reference catalog's restricted arithmetic form; never evaluate code.
    let pieces: Vec<_> = formula.split_whitespace().collect();
    if let ["level", "*", factor, "/", divisor] = pieces.as_slice()
        && let (Ok(factor), Ok(divisor)) = (factor.parse::<i64>(), divisor.parse::<i64>())
        && (0..=1_000_000).contains(&factor)
        && divisor > 0
    {
        return level.saturating_mul(factor) / divisor;
    }
    0
}

async fn menu_start_battle(
    state: &GameState,
    pool: &MySqlPool,
    args: &[Json],
    panel: &str,
    event_key: &str,
    default_energy_max: i64,
    default_bosses: &[&str],
) -> Result {
    let id = integer(args, 0);
    let boss_id = string(args, 1);
    let key = string(args, 5);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(cfg) = active_event_config(pool, panel).await? else {
        return Ok(json!({"status": 2, "result": "Event is not currently active."}));
    };
    let energy_max = cfg
        .get("energy_max")
        .and_then(Json::as_i64)
        .unwrap_or(default_energy_max);
    let row = get_or_create_event_row(pool, id, event_key, energy_max).await?;
    if row.energy <= 0 {
        return Ok(json!({"status": 2, "result": "Not enough energy."}));
    }

    if !auth::valid_legacy_hash(
        &string(args, 4),
        &format!("{id}{boss_id}{}{}", string(args, 3), string(args, 2)),
    ) {
        return Ok(json!({"status":2,"result":"Security check failed."}));
    }
    let boss = find_boss(&cfg["bosses"], &boss_id);
    if boss.is_none() && !default_bosses.contains(&boss_id.as_str()) {
        return Ok(json!({"status":2,"result":"Unknown event boss."}));
    }
    if sqlx::query("UPDATE character_event_data SET energy = energy - 1, updated_at=NOW() WHERE id=? AND energy>0")
        .bind(row.id).execute(pool).await?.rows_affected()!=1 {return Ok(json!({"status":2,"result":"Not enough energy."}));}

    let code = generate_code(10);
    let hash = auth::sha256_hex(&format!("{boss_id}{code}{id}"));
    let rewards_cfg = boss
        .filter(|v| v.is_object())
        .cloned()
        .unwrap_or_else(|| cfg.get("rewards").cloned().unwrap_or_else(|| json!({})));
    state.event_battles.lock().await.insert(
        (id, event_key.into()),
        EventBattle {
            code: code.clone(),
            boss_id: boss_id.clone(),
            started: std::time::Instant::now(),
            rewards_cfg,
        },
    );
    Ok(json!({
        "status": 1,
        "error": 0,
        "code": code,
        "hash": hash,
    }))
}

async fn menu_finish_battle(
    state: &GameState,
    pool: &MySqlPool,
    args: &[Json],
    event_key: &str,
    default_energy_max: i64,
) -> Result {
    let id = integer(args, 0);
    let code = string(args, 2);
    let win_flag = integer(args, 5);
    let key = string(args, 6);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let mut battles = state.event_battles.lock().await;
    let map_key = (id, event_key.into());
    let Some(battle) = battles.get(&map_key) else {
        return Ok(json!({"status":2,"result":"Invalid or expired battle session."}));
    };
    if battle.code != code
        || battle.boss_id != string(args, 1)
        || battle.started.elapsed().as_secs() >= 1800
    {
        return Ok(json!({"status":2,"result":"Invalid or expired battle session."}));
    }
    let expected = format!(
        "{id}{}{}{}{}",
        string(args, 1),
        code,
        string(args, 3),
        string(args, 5)
    );
    if !auth::valid_legacy_hash(&string(args, 4), &expected) {
        return Ok(json!({"status":2,"result":"Security check failed."}));
    }
    let row = get_or_create_event_row(pool, id, event_key, default_energy_max).await?;
    let (xp_awarded, gold_awarded, items_awarded, level, xp, level_up) =
        award_event_battle(pool, id, row.id, win_flag, &battle.rewards_cfg).await?;
    battles.remove(&map_key);
    Ok(json!({
        "status": 1,
        "error": 0,
        "result": [xp_awarded,gold_awarded,items_awarded],
        "level": level,
        "xp": xp,
        "level_up": level_up,
    }))
}

async fn menu_refill_energy(
    pool: &MySqlPool,
    args: &[Json],
    panel: &str,
    event_key: &str,
    default_energy_max: i64,
) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(cfg) = active_event_config(pool, panel).await? else {
        return Ok(json!({"status": 2, "result": "Event is not currently active."}));
    };
    let token_cost = cfg
        .get("refill_token_cost")
        .and_then(Json::as_i64)
        .unwrap_or(50);
    let energy_max = cfg
        .get("energy_max")
        .and_then(Json::as_i64)
        .unwrap_or(default_energy_max);

    let mut tx = pool.begin().await?;
    let Some(user_id) =
        sqlx::query_scalar::<_, u64>("SELECT user_id FROM characters WHERE id=? FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
    else {
        return Ok(session_error("Character not found"));
    };
    let tokens = i64::from(
        sqlx::query_scalar::<_, i32>("SELECT tokens FROM users WHERE id=? FOR UPDATE")
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?,
    );
    if tokens < token_cost {
        return Ok(json!({"status": 2, "result": "Not enough tokens to refill energy."}));
    }
    sqlx::query("UPDATE users SET tokens=?, updated_at=NOW() WHERE id=?")
        .bind(tokens - token_cost)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let row = get_or_create_event_row(pool, id, event_key, energy_max).await?;
    sqlx::query("UPDATE character_event_data SET energy=?, updated_at=NOW() WHERE id=?")
        .bind(energy_max)
        .bind(row.id)
        .execute(pool)
        .await?;

    Ok(json!({
        "status": 1,
        "error": 0,
        "energy": energy_max,
    }))
}

async fn menu_get_bonus_rewards(
    pool: &MySqlPool,
    args: &[Json],
    panel: &str,
    event_key: &str,
    default_energy_max: i64,
    milestones_key: &str,
) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(cfg) = active_event_config(pool, panel).await? else {
        return Ok(json!({"status": 2, "result": "Event is not currently active."}));
    };
    let count = cfg
        .get(milestones_key)
        .and_then(Json::as_array)
        .map(Vec::len)
        .unwrap_or(8);
    let row = get_or_create_event_row(pool, id, event_key, default_energy_max).await?;
    let raw_arr = row
        .milestones_claimed
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut claimed = Vec::with_capacity(count);
    for i in 0..count {
        let is_claimed = raw_arr
            .get(i)
            .and_then(Json::as_bool)
            .unwrap_or_else(|| raw_arr.iter().any(|v| v.as_u64() == Some(i as u64)));
        claimed.push(is_claimed);
    }
    let data:Vec<Json>=(0..count).map(|i|json!({"id":cfg[milestones_key][i]["id"],"req":cfg[milestones_key][i]["requirement"],"claimed":claimed[i]})).collect();
    Ok(json!({
        "status": 1,
        "error": 0,
        "Error": 0,
        "milestone": row.battles,
        "total_spins": row.battles,
        "data":data,
        "rewards": claimed,
    }))
}

async fn menu_claim_bonus_reward(
    pool: &MySqlPool,
    args: &[Json],
    panel: &str,
    event_key: &str,
    default_energy_max: i64,
    milestones_key: &str,
) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let reward_idx = integer(args, 2) as usize;
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(cfg) = active_event_config(pool, panel).await? else {
        return Ok(json!({"status": 2, "result": "Event is not currently active."}));
    };
    let milestones = cfg
        .get(milestones_key)
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_default();
    let Some(m_entry) = milestones.get(reward_idx) else {
        return Ok(json!({"status": 2, "result": "Invalid milestone index."}));
    };
    let req = m_entry
        .get("requirement")
        .and_then(Json::as_i64)
        .unwrap_or(0);
    let raw_id = m_entry
        .get("id")
        .or_else(|| m_entry.get("reward"))
        .and_then(Json::as_str)
        .unwrap_or("gold_10000");

    let row = get_or_create_event_row(pool, id, event_key, default_energy_max).await?;
    if row.battles < req {
        return Ok(
            json!({"status": 2, "result": format!("Need {req} battles to claim this reward.")}),
        );
    }
    let raw_arr = row
        .milestones_claimed
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut claimed = Vec::with_capacity(milestones.len());
    for i in 0..milestones.len() {
        let is_c = raw_arr
            .get(i)
            .and_then(Json::as_bool)
            .unwrap_or_else(|| raw_arr.iter().any(|v| v.as_u64() == Some(i as u64)));
        claimed.push(is_c);
    }
    if claimed.get(reward_idx).copied().unwrap_or(false) {
        return Ok(json!({"status": 2, "result": "Milestone already claimed."}));
    }

    let mut tx = pool.begin().await?;
    let gender: i8 = sqlx::query_scalar("SELECT gender FROM characters WHERE id=? FOR UPDATE")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    let latest: Json = sqlx::query_scalar(
        "SELECT milestones_claimed FROM character_event_data WHERE id=? FOR UPDATE",
    )
    .bind(row.id)
    .fetch_one(&mut *tx)
    .await?;
    if latest.as_array().is_some_and(|a| {
        a.get(reward_idx).and_then(Json::as_bool) == Some(true)
            || a.iter().any(|v| v.as_u64() == Some(reward_idx as u64))
    }) {
        return Ok(json!({"status":2,"result":"Milestone already claimed."}));
    }
    if let Some(latest) = latest.as_array() {
        for (index, flag) in claimed.iter_mut().enumerate() {
            *flag = latest
                .get(index)
                .and_then(Json::as_bool)
                .unwrap_or_else(|| latest.iter().any(|v| v.as_u64() == Some(index as u64)));
        }
    }
    let quantity = m_entry["quantity"].as_i64().unwrap_or(1);
    if !(1..=1000000).contains(&quantity) {
        return Ok(json!({"status":2,"result":"Invalid milestone quantity."}));
    }
    let reward = if quantity == 1 {
        raw_id.to_owned()
    } else {
        format!("{raw_id}:{quantity}")
    };
    rewards::grant(&mut tx, id, &reward).await?;
    if let Some(slot) = claimed.get_mut(reward_idx) {
        *slot = true;
    }
    sqlx::query(
        "UPDATE character_event_data SET milestones_claimed=?, updated_at=NOW() WHERE id=?",
    )
    .bind(serde_json::to_string(&claimed).unwrap_or_else(|_| "[]".into()))
    .bind(row.id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(json!({
        "status": 1,
        "error": 0,
        "reward": [resolve_gender_placeholder(&reward, gender)],
    }))
}

// ---------------------------------------------------------------------------
// 5. HalloweenEvent2025
// ---------------------------------------------------------------------------

pub async fn dispatch_halloween_2025(
    state: &GameState,
    pool: &MySqlPool,
    method: &str,
    args: &[Json],
) -> Result {
    const PANEL: &str = "HalloweenMenu";
    const KEY: &str = "halloween_2025";
    match method {
        "getBattleData" => menu_get_battle_data(pool, args, PANEL, KEY, 8).await,
        "startBattle" => {
            menu_start_battle(
                state,
                pool,
                args,
                PANEL,
                KEY,
                8,
                &["ene_2010", "ene_2011", "ene_2012"],
            )
            .await
        }
        "finishBattle" => menu_finish_battle(state, pool, args, KEY, 8).await,
        "refillEnergy" => menu_refill_energy(pool, args, PANEL, KEY, 8).await,
        "getBonusRewards" => menu_get_bonus_rewards(pool, args, PANEL, KEY, 8, "milestones").await,
        "claimBonusRewards" => {
            menu_claim_bonus_reward(pool, args, PANEL, KEY, 8, "milestones").await
        }
        "buySkill" => menu_buy_skill(pool, args, PANEL, KEY).await,
        _ => Ok(session_error("Invalid HalloweenEvent2025 method")),
    }
}

// ---------------------------------------------------------------------------
// 6. ConfrontingDeathEvent2025
// ---------------------------------------------------------------------------

pub async fn dispatch_confronting_death_2025(
    state: &GameState,
    pool: &MySqlPool,
    method: &str,
    args: &[Json],
) -> Result {
    const PANEL: &str = "ConfrontingDeathMenu";
    const KEY: &str = "confronting_death_2025";
    match method {
        "getBattleData" => menu_get_battle_data(pool, args, PANEL, KEY, 8).await,
        "startBattle" => menu_start_battle(state, pool, args, PANEL, KEY, 8, &["ene_2135"]).await,
        "finishBattle" => menu_finish_battle(state, pool, args, KEY, 8).await,
        "refillEnergy" => menu_refill_energy(pool, args, PANEL, KEY, 8).await,
        "getBonusRewards" => menu_get_bonus_rewards(pool, args, PANEL, KEY, 8, "milestones").await,
        "claimBonusRewards" => {
            menu_claim_bonus_reward(pool, args, PANEL, KEY, 8, "milestones").await
        }
        "buySkill" => menu_buy_skill(pool, args, PANEL, KEY).await,
        _ => Ok(session_error("Invalid ConfrontingDeathEvent2025 method")),
    }
}

async fn menu_buy_skill(pool: &MySqlPool, args: &[Json], panel: &str, event_key: &str) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let pkg_idx = integer(args, 2) as usize;
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(cfg) = active_event_config(pool, panel).await? else {
        return Ok(
            json!({"status": 2, "result": "Confronting Death event is not currently active."}),
        );
    };
    let packages = cfg
        .get("skill_packages")
        .or_else(|| cfg.get("training"))
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_default();
    let Some(pkg) = packages.get(pkg_idx) else {
        return Ok(json!({"status": 2, "result": "Invalid package index."}));
    };
    let skill_id = pkg
        .get("skill_id")
        .or_else(|| pkg.get("id"))
        .and_then(Json::as_str)
        .unwrap_or("");
    let cost_gold = pkg.get("cost_gold").and_then(Json::as_i64).unwrap_or(0);

    if skill_id.is_empty() {
        return Ok(json!({"status": 2, "result": "Skill package configuration error."}));
    }

    let mut tx = pool.begin().await?;
    let Some(c_row) = sqlx::query("SELECT c.user_id,c.gold,u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
    else {
        return Ok(session_error("Character not found"));
    };
    let user_id: u64 = c_row.get("user_id");
    let premium = usize::from(c_row.try_get::<i8, _>("account_type")? > 0);
    let cost_tokens = pkg["cost_tokens"]
        .as_i64()
        .or_else(|| pkg["price"].get(premium).and_then(Json::as_i64))
        .unwrap_or(0);
    if cost_tokens < 0 || cost_gold < 0 {
        return Ok(json!({"status":2,"result":"Invalid skill price."}));
    }

    let gold = i64::from(c_row.get::<i32, _>("gold"));

    let owned: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM character_skills WHERE character_id=? AND skill_id=?",
    )
    .bind(id)
    .bind(skill_id)
    .fetch_one(&mut *tx)
    .await?;
    if owned > 0 {
        return Ok(json!({"status": 2, "result": "You already own this skill."}));
    }

    if cost_tokens > 0 {
        let tokens = i64::from(
            sqlx::query_scalar::<_, i32>("SELECT tokens FROM users WHERE id=? FOR UPDATE")
                .bind(user_id)
                .fetch_one(&mut *tx)
                .await?,
        );
        if tokens < cost_tokens {
            return Ok(json!({"status": 2, "result": "Not enough tokens."}));
        }
        sqlx::query("UPDATE users SET tokens=?, updated_at=NOW() WHERE id=?")
            .bind(tokens - cost_tokens)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
    }
    if cost_gold > 0 {
        if gold < cost_gold {
            return Ok(json!({"status": 2, "result": "Not enough gold."}));
        }
        sqlx::query("UPDATE characters SET gold=?, updated_at=NOW() WHERE id=?")
            .bind(gold - cost_gold)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }

    sqlx::query(
        "INSERT INTO character_skills (character_id, skill_id, created_at, updated_at) VALUES (?, ?, NOW(), NOW())",
    )
    .bind(id)
    .bind(skill_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let row = get_or_create_event_row(pool, id, event_key, 8).await?;
    sqlx::query("UPDATE character_event_data SET bought=1, updated_at=NOW() WHERE id=?")
        .bind(row.id)
        .execute(pool)
        .await?;

    Ok(json!({"status": 1, "error": 0}))
}

// ---------------------------------------------------------------------------
// 7. ThanksGivingEvent2025
// ---------------------------------------------------------------------------

pub async fn dispatch_thanksgiving_2025(
    state: &GameState,
    pool: &MySqlPool,
    method: &str,
    args: &[Json],
) -> Result {
    const PANEL: &str = "FeastOfGratitudeMenu";
    const KEY: &str = "thanksgiving_2025";
    match method {
        "getBattleData" => menu_get_battle_data(pool, args, PANEL, KEY, 10).await,
        "startBattle" => {
            menu_start_battle(
                state,
                pool,
                args,
                PANEL,
                KEY,
                10,
                &["ene_2120", "ene_2121", "ene_2122"],
            )
            .await
        }
        "finishBattle" => menu_finish_battle(state, pool, args, KEY, 10).await,
        "refillEnergy" => menu_refill_energy(pool, args, PANEL, KEY, 10).await,
        "getBonusRewards" => menu_get_bonus_rewards(pool, args, PANEL, KEY, 10, "milestones").await,
        "claimBonusRewards" => {
            menu_claim_bonus_reward(pool, args, PANEL, KEY, 10, "milestones").await
        }
        "getPackage" => tg_get_package(pool, args).await,
        "buyPackage" => tg_buy_package(pool, args).await,
        _ => Ok(session_error("Invalid ThanksGivingEvent2025 method")),
    }
}

async fn tg_get_package(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(cfg) = active_event_config(pool, "FeastOfGratitudeMenu").await? else {
        return Ok(json!({"status": 2, "result": "Thanksgiving event is not currently active."}));
    };
    let energy_max = cfg.get("energy_max").and_then(Json::as_i64).unwrap_or(10);
    let row = get_or_create_event_row(pool, id, "thanksgiving_2025", energy_max).await?;
    Ok(json!({
        "status": 1,
        "error": 0,
        "bought": row.bought,
    }))
}

async fn tg_buy_package(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(cfg) = active_event_config(pool, "FeastOfGratitudeMenu").await? else {
        return Ok(json!({"status": 2, "result": "Thanksgiving event is not currently active."}));
    };
    let energy_max = cfg.get("energy_max").and_then(Json::as_i64).unwrap_or(10);
    let row = get_or_create_event_row(pool, id, "thanksgiving_2025", energy_max).await?;
    if row.bought {
        return Ok(json!({"status": 2, "result": "You have already purchased this package."}));
    }
    let pkg = cfg.get("package").cloned().unwrap_or_else(|| json!({}));
    let cost_tokens = pkg.get("cost_tokens").and_then(Json::as_i64).unwrap_or(500);
    let rewards_list = pkg
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
    let tokens = i64::from(
        sqlx::query_scalar::<_, i32>("SELECT tokens FROM users WHERE id=? FOR UPDATE")
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?,
    );
    if tokens < cost_tokens {
        return Ok(json!({"status": 2, "result": "Not enough tokens."}));
    }
    sqlx::query("UPDATE users SET tokens=?, updated_at=NOW() WHERE id=?")
        .bind(tokens - cost_tokens)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    let mut granted = Vec::with_capacity(rewards_list.len());
    for entry in &rewards_list {
        let raw_id = entry
            .as_str()
            .or_else(|| entry.get("id").and_then(Json::as_str))
            .unwrap_or("");
        if !raw_id.is_empty() {
            rewards::grant(&mut tx, id, raw_id).await?;
            granted.push(resolve_gender_placeholder(raw_id, gender));
        }
    }
    sqlx::query("UPDATE character_event_data SET bought=1, updated_at=NOW() WHERE id=?")
        .bind(row.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(json!({
        "status": 1,
        "error": 0,
        "rewards": granted,
    }))
}

// ---------------------------------------------------------------------------
// 8. ChristmasEvent2021 (DailyGacha)
// ---------------------------------------------------------------------------

pub async fn dispatch_christmas_2021(
    state: &GameState,
    pool: &MySqlPool,
    method: &str,
    args: &[Json],
) -> Result {
    match method {
        "getData" => xmas21_get_data(pool, args).await,
        "getGachaRewards" => xmas21_get_gacha_rewards(state, pool, args).await,
        "getBonusRewards" => xmas21_get_bonus_rewards(pool, args).await,
        "claimBonusGachaRewards" => xmas21_claim_bonus_rewards(pool, args).await,
        "getRewardList" => xmas21_get_reward_list(pool, args).await,
        "getPersonalGachaHistory" => xmas21_get_personal_history(pool, args).await,
        "getGlobalGachaHistory" => xmas21_get_global_history(state, pool, args).await,
        _ => Ok(session_error("Invalid ChristmasEvent2021 method")),
    }
}

async fn xmas21_get_data(pool: &MySqlPool, args: &[Json]) -> Result {
    let (id, key) = extract_id_key(args);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(cfg) = active_event_config(pool, "DailyGacha").await? else {
        return Ok(json!({"status": 2, "result": "Daily Gacha is not currently active."}));
    };
    let coin_id = cfg
        .get("coin_id")
        .and_then(Json::as_str)
        .unwrap_or("material_874");
    let coin_qty = sqlx::query_scalar::<_, i32>(
        "SELECT quantity FROM character_items WHERE character_id=? AND item_id=?",
    )
    .bind(id)
    .bind(coin_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or(0);
    Ok(json!({
        "status": 1,
        "error": 0,
        "coin": coin_qty,
    }))
}

async fn xmas21_get_gacha_rewards(state: &GameState, pool: &MySqlPool, args: &[Json]) -> Result {
    let (id, key) = extract_id_key(args);
    let play_type = string(args, 2);
    let play_qty = integer(args, 3).clamp(1, 10);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(cfg) = active_event_config(pool, "DailyGacha").await? else {
        return Ok(json!({"status": 2, "result": "Daily Gacha is not currently active."}));
    };
    let coin_id = cfg
        .get("coin_id")
        .and_then(Json::as_str)
        .unwrap_or("material_874");
    let tier_idx = if play_qty >= 10 {
        2
    } else if play_qty >= 2 {
        1
    } else {
        0
    };
    let pool_items = cfg
        .get("pool")
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!({"id": "gold_10000", "weight": 100})]);

    let mut tx = pool.begin().await?;
    let Some(c_row) =
        sqlx::query("SELECT user_id, name, level, gender FROM characters WHERE id=? FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
    else {
        return Ok(session_error("Character not found"));
    };
    let user_id: u64 = c_row.get("user_id");
    let char_name: String = c_row.get("name");
    let char_level: i32 = c_row.get("level");
    let gender: i8 = c_row.get("gender");

    if play_type == "coins" {
        let coin_costs = cfg
            .get("coin_costs")
            .and_then(Json::as_array)
            .cloned()
            .unwrap_or_else(|| vec![json!(1), json!(2), json!(10)]);
        let needed = coin_costs
            .get(tier_idx)
            .and_then(Json::as_i64)
            .unwrap_or(play_qty);
        let m_row = sqlx::query(
            "SELECT id, quantity FROM character_items WHERE character_id=? AND item_id=? FOR UPDATE",
        )
        .bind(id)
        .bind(coin_id)
        .fetch_optional(&mut *tx)
        .await?;
        let (m_id, have) = match m_row {
            Some(r) => (
                r.get::<u64, _>("id"),
                i64::from(r.get::<i32, _>("quantity")),
            ),
            None => (0, 0),
        };
        if have < needed {
            return Ok(json!({"status": 2, "result": "Not enough gacha coins."}));
        }
        if have == needed {
            sqlx::query("DELETE FROM character_items WHERE id=?")
                .bind(m_id)
                .execute(&mut *tx)
                .await?;
        } else {
            sqlx::query("UPDATE character_items SET quantity=?, updated_at=NOW() WHERE id=?")
                .bind(have - needed)
                .bind(m_id)
                .execute(&mut *tx)
                .await?;
        }
    } else {
        let token_costs = cfg
            .get("token_costs")
            .and_then(Json::as_array)
            .cloned()
            .unwrap_or_else(|| vec![json!(30), json!(60), json!(300)]);
        let needed = token_costs
            .get(tier_idx)
            .and_then(Json::as_i64)
            .unwrap_or(30 * play_qty);
        let tokens = i64::from(
            sqlx::query_scalar::<_, i32>("SELECT tokens FROM users WHERE id=? FOR UPDATE")
                .bind(user_id)
                .fetch_one(&mut *tx)
                .await?,
        );
        if tokens < needed {
            return Ok(json!({"status": 2, "result": "Not enough tokens."}));
        }
        sqlx::query("UPDATE users SET tokens=?, updated_at=NOW() WHERE id=?")
            .bind(tokens - needed)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
    }

    let mut granted = Vec::with_capacity(play_qty as usize);
    for _ in 0..play_qty {
        let picked = weighted_pick(&pool_items);
        rewards::grant(&mut tx, id, &picked).await?;
        granted.push(resolve_gender_placeholder(&picked, gender));
    }
    tx.commit().await?;

    let row = get_or_create_event_row(pool, id, "daily_gacha", 0).await?;
    let mut extra = row.extra.as_object().cloned().unwrap_or_default();
    let mut p_hist = extra
        .get("history")
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_default();
    let obtained_at = now_hm();
    for r_id in &granted {
        p_hist.insert(
            0,
            json!({
                "id": id,
                "name": char_name,
                "level": char_level,
                "reward": r_id,
                "spin": play_qty,
                "obtained_at": obtained_at,
            }),
        );
    }
    p_hist.truncate(50);
    extra.insert("history".into(), Json::Array(p_hist));
    sqlx::query(
        "UPDATE character_event_data SET battles = battles + ?, extra=?, updated_at=NOW() WHERE id=?",
    )
    .bind(play_qty)
    .bind(serde_json::to_string(&extra).unwrap_or_else(|_| "{}".into()))
    .bind(row.id)
    .execute(pool)
    .await?;

    let mut g_map = state.gacha_histories.lock().await;
    let g_hist = g_map.entry("daily_gacha".into()).or_default();
    for r_id in &granted {
        g_hist.insert(
            0,
            json!({
                "id": id,
                "name": char_name,
                "level": char_level,
                "reward": r_id,
                "spin": play_qty,
                "obtained_at": obtained_at,
            }),
        );
    }
    g_hist.truncate(100);

    let remaining_coins = sqlx::query_scalar::<_, i32>(
        "SELECT quantity FROM character_items WHERE character_id=? AND item_id=?",
    )
    .bind(id)
    .bind(coin_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or(0);

    Ok(json!({
        "status": 1,
        "error": 0,
        "rewards": granted,
        "coin": remaining_coins,
    }))
}

async fn xmas21_get_bonus_rewards(pool: &MySqlPool, args: &[Json]) -> Result {
    let (id, key) = extract_id_key(args);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(cfg) = active_event_config(pool, "DailyGacha").await? else {
        return Ok(json!({"status": 2, "result": "Daily Gacha is not currently active."}));
    };
    let count = cfg
        .get("bonus_milestones")
        .and_then(Json::as_array)
        .map(Vec::len)
        .unwrap_or(8);
    let row = get_or_create_event_row(pool, id, "daily_gacha", 0).await?;
    let raw_arr = row
        .milestones_claimed
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut claimed = Vec::with_capacity(count);
    for i in 0..count {
        let is_c = raw_arr
            .get(i)
            .and_then(Json::as_bool)
            .unwrap_or_else(|| raw_arr.iter().any(|v| v.as_u64() == Some(i as u64)));
        claimed.push(is_c);
    }
    Ok(json!({
        "status": 1,
        "error": 0,
        "total_spin": row.battles,
        "rewards": claimed,
    }))
}

async fn xmas21_claim_bonus_rewards(pool: &MySqlPool, args: &[Json]) -> Result {
    let (id, key) = extract_id_key(args);
    let reward_idx = integer(args, 2) as usize;
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(cfg) = active_event_config(pool, "DailyGacha").await? else {
        return Ok(json!({"status": 2, "result": "Daily Gacha is not currently active."}));
    };
    let milestones = cfg
        .get("bonus_milestones")
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_default();
    let Some(m_entry) = milestones.get(reward_idx) else {
        return Ok(json!({"status": 2, "result": "Invalid bonus reward index."}));
    };
    let req = m_entry
        .get("requirement")
        .and_then(Json::as_i64)
        .unwrap_or(0);
    let raw_id = m_entry
        .get("id")
        .or_else(|| m_entry.get("reward"))
        .and_then(Json::as_str)
        .unwrap_or("gold_10000");

    let row = get_or_create_event_row(pool, id, "daily_gacha", 0).await?;
    if row.battles < req {
        return Ok(
            json!({"status": 2, "result": format!("Need {req} spins to claim this bonus.")}),
        );
    }
    let raw_arr = row
        .milestones_claimed
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut claimed = Vec::with_capacity(milestones.len());
    for i in 0..milestones.len() {
        let is_c = raw_arr
            .get(i)
            .and_then(Json::as_bool)
            .unwrap_or_else(|| raw_arr.iter().any(|v| v.as_u64() == Some(i as u64)));
        claimed.push(is_c);
    }
    if claimed.get(reward_idx).copied().unwrap_or(false) {
        return Ok(json!({"status": 2, "result": "Bonus reward already claimed."}));
    }

    let mut tx = pool.begin().await?;
    let gender: i8 = sqlx::query_scalar("SELECT gender FROM characters WHERE id=? FOR UPDATE")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    rewards::grant(&mut tx, id, raw_id).await?;
    if let Some(slot) = claimed.get_mut(reward_idx) {
        *slot = true;
    }
    sqlx::query(
        "UPDATE character_event_data SET milestones_claimed=?, updated_at=NOW() WHERE id=?",
    )
    .bind(serde_json::to_string(&claimed).unwrap_or_else(|_| "[]".into()))
    .bind(row.id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(json!({
        "status": 1,
        "error": 0,
        "rewards": [resolve_gender_placeholder(raw_id, gender)],
    }))
}

async fn xmas21_get_reward_list(pool: &MySqlPool, args: &[Json]) -> Result {
    let (id, key) = extract_id_key(args);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(cfg) = active_event_config(pool, "DailyGacha").await? else {
        return Ok(json!({"status": 2, "result": "Daily Gacha is not currently active."}));
    };
    let gender: i8 = sqlx::query_scalar("SELECT gender FROM characters WHERE id=?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .unwrap_or(0);
    let tiers = cfg
        .get("reward_tiers")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let resolve_list = |key_name: &str| -> Vec<String> {
        tiers
            .get(key_name)
            .and_then(Json::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Json::as_str)
                    .map(|s| resolve_gender_placeholder(s, gender))
                    .collect()
            })
            .unwrap_or_default()
    };
    let top = resolve_list("top");
    let mid = resolve_list("mid");
    let common = resolve_list("common");
    let bonus: Vec<String> = cfg
        .get("bonus_milestones")
        .and_then(Json::as_array)
        .map(|arr| {
            arr.iter()
                .map(|m| {
                    let raw = m
                        .get("id")
                        .or_else(|| m.get("reward"))
                        .and_then(Json::as_str)
                        .unwrap_or("gold_10000");
                    resolve_gender_placeholder(raw, gender)
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(json!({
        "status": 1,
        "error": 0,
        "rewards": [top, mid, common],
        "milestone": bonus,
    }))
}

async fn xmas21_get_personal_history(pool: &MySqlPool, args: &[Json]) -> Result {
    let (id, key) = extract_id_key(args);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let row = get_or_create_event_row(pool, id, "daily_gacha", 0).await?;
    let histories = row
        .extra
        .get("history")
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(json!({
        "status": 1,
        "error": 0,
        "histories": histories,
    }))
}

async fn xmas21_get_global_history(state: &GameState, pool: &MySqlPool, args: &[Json]) -> Result {
    let (id, key) = extract_id_key(args);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let histories = state
        .gacha_histories
        .lock()
        .await
        .get("daily_gacha")
        .cloned()
        .unwrap_or_default();
    Ok(json!({
        "status": 1,
        "error": 0,
        "histories": histories,
    }))
}

// ---------------------------------------------------------------------------
// 9. ChristmasEvent2025 & NewYear2026
// ---------------------------------------------------------------------------

pub async fn dispatch_christmas_2025(
    state: &GameState,
    pool: &MySqlPool,
    method: &str,
    args: &[Json],
) -> Result {
    const PANEL: &str = "ChristmasMenu";
    const KEY: &str = "christmas_2025";
    match method {
        "getBattleData" => menu_get_battle_data(pool, args, PANEL, KEY, 10).await,
        "startBattle" => {
            menu_start_battle(
                state,
                pool,
                args,
                PANEL,
                KEY,
                10,
                &[
                    "ene_2125", "ene_2126", "ene_2127", "ene_2128", "ene_2129", "ene_2130",
                ],
            )
            .await
        }
        "finishBattle" => menu_finish_battle(state, pool, args, KEY, 10).await,
        "refillEnergy" => menu_refill_energy(pool, args, PANEL, KEY, 10).await,
        "getBonusRewards" => menu_get_bonus_rewards(pool, args, PANEL, KEY, 10, "milestones").await,
        "claimBonusRewards" => {
            menu_claim_bonus_reward(pool, args, PANEL, KEY, 10, "milestones").await
        }
        "getMinigameData" => xmas25_get_minigame_data(pool, args).await,
        "startMinigame" => xmas25_start_minigame(state, pool, args).await,
        "finishMinigame" => xmas25_finish_minigame(state, pool, args).await,
        "refillMinigameEnergy" => xmas25_refill_minigame(pool, args).await,
        "getGachaData" => xmas25_get_gacha_data(pool, args).await,
        "getGachaRewards" => xmas25_get_gacha_rewards(state, pool, args).await,
        "getBonusGachaRewards" => {
            menu_get_bonus_rewards(
                pool,
                args,
                PANEL,
                "christmas_2025_gacha",
                0,
                "gacha_milestones",
            )
            .await
        }
        "claimBonusGachaRewards" => {
            menu_claim_bonus_reward(
                pool,
                args,
                PANEL,
                "christmas_2025_gacha",
                0,
                "gacha_milestones",
            )
            .await
        }
        "getPersonalGachaHistory" => xmas25_get_personal_history(pool, args).await,
        "getGlobalGachaHistory" => xmas25_get_global_history(state, pool, args).await,
        _ => Ok(session_error("Invalid ChristmasEvent2025 method")),
    }
}

async fn xmas25_get_minigame_data(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(cfg) = active_event_config(pool, "ChristmasMenu").await? else {
        return Ok(json!({"status": 2, "result": "Christmas event is not currently active."}));
    };
    let energy_max = cfg
        .get("minigame")
        .and_then(|m| m.get("energy_max"))
        .and_then(Json::as_i64)
        .unwrap_or(8);
    let row = get_or_create_event_row(pool, id, "christmas_2025_mini", energy_max).await?;
    Ok(json!({
        "status": 1,
        "error": 0,
        "energy": row.energy,
    }))
}

async fn xmas25_start_minigame(state: &GameState, pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(e) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(e);
    }
    let Some(cfg) = active_event_config(pool, "ChristmasMenu").await? else {
        return Ok(json!({"status":2,"result":"Christmas event is not active."}));
    };
    let energy_max = cfg["minigame"]["energy_max"].as_i64().unwrap_or(8);
    let row = get_or_create_event_row(pool, id, "christmas_2025_mini", energy_max).await?;
    let code = authentication::mission_token()
        .map_err(|_| sqlx::Error::Protocol("Entropy unavailable".into()))?;
    let mut sessions = state.event_battles.lock().await;
    if sqlx::query(
        "UPDATE character_event_data SET energy=energy-1,updated_at=NOW() WHERE id=? AND energy>0",
    )
    .bind(row.id)
    .execute(pool)
    .await?
    .rows_affected()
        != 1
    {
        return Ok(json!({"status":2,"result":"Not enough minigame energy."}));
    }
    let rewards = cfg["minigame"]
        .as_array()
        .cloned()
        .or_else(|| cfg["minigame"]["rewards"].as_array().cloned())
        .unwrap_or_default();
    sessions.insert(
        (id, "christmas_2025_mini".into()),
        EventBattle {
            code: code.clone(),
            boss_id: "minigame".into(),
            started: std::time::Instant::now(),
            rewards_cfg: json!(rewards),
        },
    );
    Ok(json!({"status":1,"code":code,"energy":row.energy-1}))
}
async fn xmas25_finish_minigame(state: &GameState, pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let score = integer(args, 2);
    if !(0..=100000).contains(&score) {
        return Ok(json!({"status":2,"result":"Invalid score."}));
    }
    let mut sessions = state.event_battles.lock().await;
    let map_key = (id, "christmas_2025_mini".into());
    let Some(session) = sessions.get(&map_key) else {
        return Ok(json!({"status":2,"result":"Start a minigame first."}));
    };
    if session.started.elapsed().as_secs() > 3600
        || string(args, 3) != auth::sha256_hex(&format!("{id}|{key}|{score}|{}", session.code))
    {
        return Ok(json!({"status":2,"result":"Invalid minigame session."}));
    }
    let mut tx = pool.begin().await?;
    let gender: i8 = sqlx::query_scalar("SELECT gender FROM characters WHERE id=? FOR UPDATE")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    let mut granted = vec![];
    for entry in session.rewards_cfg.as_array().into_iter().flatten() {
        if let Some(reward) = entry.as_str() {
            rewards::grant(&mut tx, id, reward).await?;
            granted.push(resolve_gender_placeholder(reward, gender));
        }
    }
    sqlx::query("UPDATE character_event_data SET battles=battles+1,updated_at=NOW() WHERE character_id=? AND event_key='christmas_2025_mini'").bind(id).execute(&mut *tx).await?;
    let energy:i64=sqlx::query_scalar("SELECT CAST(energy AS SIGNED) FROM character_event_data WHERE character_id=? AND event_key='christmas_2025_mini'").bind(id).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    sessions.remove(&map_key);
    Ok(json!({"status":1,"rewards":granted,"current_energy":energy}))
}

async fn xmas25_refill_minigame(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(cfg) = active_event_config(pool, "ChristmasMenu").await? else {
        return Ok(json!({"status": 2, "result": "Christmas event is not currently active."}));
    };
    let mini_cfg = cfg.get("minigame").cloned().unwrap_or_else(|| json!({}));
    let token_cost = mini_cfg
        .get("refill_token_cost")
        .and_then(Json::as_i64)
        .unwrap_or(50);
    let energy_max = mini_cfg
        .get("energy_max")
        .and_then(Json::as_i64)
        .unwrap_or(8);

    let mut tx = pool.begin().await?;
    let Some(user_id) =
        sqlx::query_scalar::<_, u64>("SELECT user_id FROM characters WHERE id=? FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
    else {
        return Ok(session_error("Character not found"));
    };
    let tokens = i64::from(
        sqlx::query_scalar::<_, i32>("SELECT tokens FROM users WHERE id=? FOR UPDATE")
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?,
    );
    if tokens < token_cost {
        return Ok(json!({"status": 2, "result": "Not enough tokens to refill energy."}));
    }
    sqlx::query("UPDATE users SET tokens=?, updated_at=NOW() WHERE id=?")
        .bind(tokens - token_cost)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let row = get_or_create_event_row(pool, id, "christmas_2025_mini", energy_max).await?;
    sqlx::query("UPDATE character_event_data SET energy=?, updated_at=NOW() WHERE id=?")
        .bind(energy_max)
        .bind(row.id)
        .execute(pool)
        .await?;

    Ok(json!({
        "status": 1,
        "error": 0,
        "energy": energy_max,
    }))
}

async fn xmas25_get_gacha_data(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    if active_event_config(pool, "ChristmasMenu").await?.is_none() {
        return Ok(json!({"status": 2, "result": "Christmas event is not currently active."}));
    }
    let coin:i64=sqlx::query_scalar("SELECT COALESCE(CAST(SUM(quantity) AS SIGNED),0) FROM character_items WHERE character_id=? AND item_id='material_2231'").bind(id).fetch_one(pool).await?;
    Ok(json!({"status":1,"error":0,"coin":coin}))
}

async fn xmas25_get_gacha_rewards(state: &GameState, pool: &MySqlPool, args: &[Json]) -> Result {
    let (id, key) = extract_id_key(args);
    let play_type = string(args, 2);
    let play_qty = integer(args, 3);
    if !matches!(play_qty, 1 | 3 | 6)
        || !matches!(play_type.as_str(), "coins" | "tokens")
        || (play_type == "coins" && play_qty == 6)
    {
        return Ok(json!({"status":2,"result":"Invalid draw selection."}));
    }
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(cfg) = active_event_config(pool, "ChristmasMenu").await? else {
        return Ok(json!({"status": 2, "result": "Christmas event is not currently active."}));
    };
    let gacha = cfg.get("gacha").cloned().unwrap_or_else(|| json!({}));
    let coin_id = gacha
        .get("coin_id")
        .and_then(Json::as_str)
        .unwrap_or("material_2231");
    let tier_idx = match play_qty {
        1 => 0,
        3 => 1,
        _ => 2,
    };
    let mut pool_items = gacha["pool"].as_array().cloned().unwrap_or_default();
    if pool_items.is_empty() {
        let weights = [5, 25, 70];
        for (tier, weight) in ["top", "mid", "common"].iter().zip(weights) {
            if let Some(items) = gacha[*tier].as_array() {
                for id in items.iter().filter_map(Json::as_str) {
                    if id.ends_with('_') {
                        continue;
                    }
                    pool_items.push(
                        json!({"id":id,"weight":(weight*10000/(items.len().max(1) as i64)).max(1)}),
                    );
                }
            }
        }
    }
    if pool_items.is_empty() {
        return Ok(json!({"status":2,"result":"Gacha rewards are not configured."}));
    }
    let mut tx = pool.begin().await?;
    let Some(c_row) =
        sqlx::query("SELECT user_id, name, level, gender FROM characters WHERE id=? FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
    else {
        return Ok(session_error("Character not found"));
    };
    let user_id: u64 = c_row.get("user_id");
    let char_name: String = c_row.get("name");
    let char_level: i32 = c_row.get("level");
    let gender: i8 = c_row.get("gender");

    if play_type == "coins" {
        let coin_costs = gacha
            .get("coin_costs")
            .and_then(Json::as_array)
            .cloned()
            .unwrap_or_else(|| vec![json!(1), json!(3), json!(6)]);
        let needed = coin_costs
            .get(tier_idx)
            .and_then(Json::as_i64)
            .unwrap_or(play_qty);
        let m_row = sqlx::query(
            "SELECT id, quantity FROM character_items WHERE character_id=? AND item_id=? FOR UPDATE",
        )
        .bind(id)
        .bind(coin_id)
        .fetch_optional(&mut *tx)
        .await?;
        let (m_id, have) = match m_row {
            Some(r) => (
                r.get::<u64, _>("id"),
                i64::from(r.get::<i32, _>("quantity")),
            ),
            None => (0, 0),
        };
        if have < needed {
            return Ok(json!({"status": 2, "result": "Not enough gacha coins."}));
        }
        if have == needed {
            sqlx::query("DELETE FROM character_items WHERE id=?")
                .bind(m_id)
                .execute(&mut *tx)
                .await?;
        } else {
            sqlx::query("UPDATE character_items SET quantity=?, updated_at=NOW() WHERE id=?")
                .bind(have - needed)
                .bind(m_id)
                .execute(&mut *tx)
                .await?;
        }
    } else {
        let token_costs = gacha
            .get("token_costs")
            .and_then(Json::as_array)
            .cloned()
            .unwrap_or_else(|| vec![json!(20), json!(50), json!(100)]);
        let needed = token_costs
            .get(tier_idx)
            .and_then(Json::as_i64)
            .unwrap_or(50 * play_qty);
        let tokens = i64::from(
            sqlx::query_scalar::<_, i32>("SELECT tokens FROM users WHERE id=? FOR UPDATE")
                .bind(user_id)
                .fetch_one(&mut *tx)
                .await?,
        );
        if tokens < needed {
            return Ok(json!({"status": 2, "result": "Not enough tokens."}));
        }
        sqlx::query("UPDATE users SET tokens=?, updated_at=NOW() WHERE id=?")
            .bind(tokens - needed)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
    }

    let mut granted = Vec::with_capacity(play_qty as usize);
    for _ in 0..play_qty {
        let picked = weighted_pick(&pool_items);
        rewards::grant(&mut tx, id, &picked).await?;
        granted.push(resolve_gender_placeholder(&picked, gender));
    }
    tx.commit().await?;

    let row = get_or_create_event_row(pool, id, "christmas_2025_gacha", 0).await?;
    let mut extra = row.extra.as_object().cloned().unwrap_or_default();
    let mut p_hist = extra
        .get("history")
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_default();
    let obtained_at = now_hm();
    for r_id in &granted {
        p_hist.insert(
            0,
            json!({
                "id": id,
                "name": char_name,
                "level": char_level,
                "reward": r_id,
                "spin": play_qty,
                "obtained_at": obtained_at,
            }),
        );
    }
    p_hist.truncate(50);
    extra.insert("history".into(), Json::Array(p_hist));
    sqlx::query(
        "UPDATE character_event_data SET battles = battles + ?, extra=?, updated_at=NOW() WHERE id=?",
    )
    .bind(play_qty)
    .bind(serde_json::to_string(&extra).unwrap_or_else(|_| "{}".into()))
    .bind(row.id)
    .execute(pool)
    .await?;

    let mut g_map = state.gacha_histories.lock().await;
    let g_hist = g_map.entry("christmas_2025".into()).or_default();
    for r_id in &granted {
        g_hist.insert(
            0,
            json!({
                "id": id,
                "name": char_name,
                "level": char_level,
                "reward": r_id,
                "spin": play_qty,
                "obtained_at": obtained_at,
            }),
        );
    }
    g_hist.truncate(100);

    let coin:i64=sqlx::query_scalar("SELECT COALESCE(CAST(SUM(quantity) AS SIGNED),0) FROM character_items WHERE character_id=? AND item_id=?").bind(id).bind(coin_id).fetch_one(pool).await?;
    Ok(json!({
        "status": 1,
        "error": 0,
        "coin":coin,
        "rewards": granted,
    }))
}

async fn xmas25_get_personal_history(pool: &MySqlPool, args: &[Json]) -> Result {
    let (id, key) = extract_id_key(args);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let row = get_or_create_event_row(pool, id, "christmas_2025_gacha", 0).await?;
    let histories = row
        .extra
        .get("history")
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(json!({
        "status": 1,
        "error": 0,
        "histories": histories,
    }))
}

async fn xmas25_get_global_history(state: &GameState, pool: &MySqlPool, args: &[Json]) -> Result {
    let (id, key) = extract_id_key(args);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let histories = state
        .gacha_histories
        .lock()
        .await
        .get("christmas_2025")
        .cloned()
        .unwrap_or_default();
    Ok(json!({
        "status": 1,
        "error": 0,
        "histories": histories,
    }))
}

pub async fn dispatch_new_year_2026(pool: &MySqlPool, method: &str, args: &[Json]) -> Result {
    match method {
        "claim" => new_year_claim(pool, args).await,
        _ => Ok(session_error("Invalid NewYear2026 method")),
    }
}

async fn new_year_claim(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(error) = guard(pool, id, &key, true).await? {
        return Ok(error);
    }
    let Some(cfg) = active_event_config(pool, "ChristmasMenu").await? else {
        return Ok(json!({"status": 2, "result": "New Year 2026 event is not currently active."}));
    };
    let rewards_list = cfg
        .get("new_year_rewards")
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_default();
    if rewards_list.is_empty() {
        return Ok(json!({"status": 2, "result": "No New Year rewards configured."}));
    }
    let row = get_or_create_event_row(pool, id, "christmas_2025", 10).await?;
    let mut extra = row.extra.as_object().cloned().unwrap_or_default();
    if extra
        .get("new_year_2026_claimed")
        .and_then(Json::as_bool)
        .unwrap_or(false)
    {
        return Ok(
            json!({"status": 2, "result": "You have already claimed the New Year 2026 gift."}),
        );
    }

    let mut tx = pool.begin().await?;
    let gender: i8 = sqlx::query_scalar("SELECT gender FROM characters WHERE id=? FOR UPDATE")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    let mut granted = Vec::with_capacity(rewards_list.len());
    for entry in &rewards_list {
        let raw_id = entry
            .as_str()
            .or_else(|| entry.get("id").and_then(Json::as_str))
            .unwrap_or("");
        if !raw_id.is_empty() {
            rewards::grant(&mut tx, id, raw_id).await?;
            granted.push(resolve_gender_placeholder(raw_id, gender));
        }
    }
    extra.insert("new_year_2026_claimed".into(), json!(true));
    sqlx::query("UPDATE character_event_data SET extra=?, updated_at=NOW() WHERE id=?")
        .bind(serde_json::to_string(&extra).unwrap_or_else(|_| "{}".into()))
        .bind(row.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(json!({
        "status": 1,
        "error": 0,
        "reward": granted,
    }))
}
