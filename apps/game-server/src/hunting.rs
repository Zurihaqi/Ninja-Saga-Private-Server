//! Hunting House and Eudemon Garden AMF services.
//! Source: `app/Services/Amf/HuntingHouseService/*` and `app/Services/Amf/EudemonGardenService/*`.
use crate::{
    GameState,
    account::{guard, integer, string},
    inventory, missions, rewards,
};
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};
use std::time::{Duration, Instant};

type Result = std::result::Result<Json, sqlx::Error>;

pub struct HuntingBattle {
    pub code: String,
    pub reward_xp: i64,
    pub reward_gold: i64,
    pub rewards: Vec<Json>,
    pub started: Instant,
}

fn error(message: &str) -> Json {
    json!({"status":0,"error":message})
}

async fn game_config(
    pool: &MySqlPool,
    key: &str,
) -> std::result::Result<Option<Json>, sqlx::Error> {
    let raw: Option<String> =
        sqlx::query_scalar("SELECT CAST(value AS CHAR) FROM game_configs WHERE `key`=? LIMIT 1")
            .bind(key)
            .fetch_optional(pool)
            .await?;
    Ok(raw.and_then(|s| serde_json::from_str(&s).ok()))
}

fn rand_inclusive(min: u32, max: u32) -> u32 {
    if min >= max {
        return min;
    }
    let span = (max - min) + 1;
    let mut bytes = [0u8; 4];
    if getrandom::fill(&mut bytes).is_err() {
        return min;
    }
    min + (u32::from_le_bytes(bytes) % span)
}

// -----------------------------------------------------------------------------
// HuntingHouse.*
// -----------------------------------------------------------------------------

pub async fn dispatch_hunting_house(
    state: &GameState,
    pool: &MySqlPool,
    method: &str,
    args: &[Json],
) -> Result {
    match method {
        "getData" => hunting_get_data(pool, args).await,
        "dailyClaim" => hunting_daily_claim(pool, args).await,
        "buyMaterial" => hunting_buy_material(pool, args).await,
        "startHunting" => hunting_start(state, pool, args).await,
        "finishHunting" => hunting_finish(state, pool, args).await,
        "getItems" => hunting_get_items(pool, args).await,
        "forgeItem" => hunting_forge_item(pool, args).await,
        _ => Ok(error("Unknown HuntingHouse operation")),
    }
}

async fn hunting_get_data(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let Some(row) = sqlx::query(
        "SELECT CAST(hunting_house_date AS CHAR) AS hh_date, CAST(CURDATE() AS CHAR) AS today FROM characters WHERE id=?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(error("Character not found"));
    };
    let hh_date: Option<String> = row.try_get("hh_date")?;
    let today: String = row.try_get("today")?;
    let daily_claim = hh_date.as_deref() == Some(today.as_str());

    let material: i32 = sqlx::query_scalar(
        "SELECT quantity FROM character_items WHERE character_id=? AND item_id='material_509' LIMIT 1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .unwrap_or(0);

    let zones = json!([
        {"easyBoss": ["ene_81"], "hardBoss": null},
        {"easyBoss": null, "hardBoss": ["ene_82", "ene_83"]},
        {"easyBoss": ["ene_84"], "hardBoss": null},
        {"easyBoss": null, "hardBoss": ["ene_120", "ene_155"]},
        {"easyBoss": null, "hardBoss": ["ene_106", "ene_101"]}
    ]);
    let bosses = json!({
        "ene_81":  {"name": "Ginkotsu",          "description": "A mechanical terror.",  "rewards": ["material_509"]},
        "ene_82":  {"name": "Shikigami Yanki",    "description": "A paper spirit.",       "rewards": ["material_509"]},
        "ene_83":  {"name": "Gedo Sessho Seki",   "description": "The killing stone.",    "rewards": ["material_509"]},
        "ene_84":  {"name": "Tengu - Fire",       "description": "Fire Tengu.",           "rewards": ["material_509"]},
        "ene_120": {"name": "Battle Turtle",      "description": "Giant Turtle.",         "rewards": ["material_509"]},
        "ene_155": {"name": "Soul General Mutoh", "description": "Undead General.",       "rewards": ["material_509"]},
        "ene_106": {"name": "Ape King",           "description": "King of Apes.",         "rewards": ["material_509"]},
        "ene_101": {"name": "Yamata no Orochi",   "description": "Eight-headed serpent.", "rewards": ["material_509"]}
    });

    Ok(json!({
        "status": 1,
        "material": material,
        "daily_claim": daily_claim,
        "zones": zones,
        "bosses": bosses
    }))
}

async fn hunting_daily_claim(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let settings = game_config(pool, "hunting_house_settings")
        .await?
        .unwrap_or_else(|| json!({}));
    let free_amount = settings["daily_claim_free"].as_i64().unwrap_or(5);
    let premium_amount = settings["daily_claim_premium"].as_i64().unwrap_or(10);

    let mut tx = pool.begin().await?;
    let Some(row) = sqlx::query(
        "SELECT CAST(c.hunting_house_date AS CHAR) AS hh_date, CAST(CURDATE() AS CHAR) AS today, u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(error("Character not found"));
    };
    let hh_date: Option<String> = row.try_get("hh_date")?;
    let today: String = row.try_get("today")?;
    if hh_date.as_deref() == Some(today.as_str()) {
        return Ok(json!({"status":2,"result":"Already claimed today!"}));
    }
    let account_type: i8 = row.try_get("account_type")?;
    let amount = if account_type == 1 {
        premium_amount
    } else {
        free_amount
    };

    sqlx::query("UPDATE characters SET hunting_house_date=CURDATE(),updated_at=NOW() WHERE id=?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    rewards::give_item(&mut tx, id, "material_509", amount).await?;
    let material: i32 = sqlx::query_scalar(
        "SELECT quantity FROM character_items WHERE character_id=? AND item_id='material_509' LIMIT 1",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .unwrap_or(0);
    tx.commit().await?;

    Ok(json!({
        "status": 1,
        "material": material
    }))
}

async fn hunting_buy_material(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let amount = integer(args, 2);
    if amount <= 0 {
        return Ok(json!({"status":2,"result":"Invalid amount!"}));
    }
    let settings = game_config(pool, "hunting_house_settings")
        .await?
        .unwrap_or_else(|| json!({}));
    let price = settings["material_price"].as_i64().unwrap_or(5);
    let Some(total_cost) = price.checked_mul(amount).filter(|c| *c >= 0) else {
        return Ok(json!({"status":2,"result":"Not enough tokens!"}));
    };

    let mut tx = pool.begin().await?;
    let Some(row) = sqlx::query(
        "SELECT c.user_id, u.tokens FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(error("Character not found"));
    };
    let tokens = i64::from(row.try_get::<i32, _>("tokens")?);
    if tokens < total_cost {
        return Ok(json!({"status":2,"result":"Not enough tokens!"}));
    }
    let user_id: u64 = row.try_get("user_id")?;
    sqlx::query("UPDATE users SET tokens=tokens-?,updated_at=NOW() WHERE id=?")
        .bind(total_cost)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    rewards::give_item(&mut tx, id, "material_509", amount).await?;
    let material: i32 = sqlx::query_scalar(
        "SELECT quantity FROM character_items WHERE character_id=? AND item_id='material_509' LIMIT 1",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .unwrap_or(0);
    tx.commit().await?;

    Ok(json!({
        "status": 1,
        "material": material
    }))
}

async fn hunting_zone_rewards(
    pool: &MySqlPool,
    zone_id: i64,
) -> std::result::Result<Vec<Json>, sqlx::Error> {
    if let Some(config) = game_config(pool, "hunting_house_zone_rewards").await? {
        let key = zone_id.to_string();
        if let Some(arr) = config.get(&key).and_then(Json::as_array) {
            return Ok(arr.clone());
        }
        if let Some(arr) = config
            .as_array()
            .and_then(|list| list.get(usize::try_from(zone_id).unwrap_or(usize::MAX)))
            .and_then(Json::as_array)
        {
            return Ok(arr.clone());
        }
    }
    let default: &[&str] = match zone_id {
        1 | 3 => &["material_509"],
        2 => &["material_509", "material_509"],
        4 => &["material_509", "material_509", "material_509"],
        5 => &[
            "material_509",
            "material_509",
            "material_509",
            "material_509",
        ],
        _ => &[],
    };
    Ok(default.iter().map(|s| json!(s)).collect())
}

async fn hunting_start(state: &GameState, pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let zone_str = string(args, 1);
    let zone_id = integer(args, 1);
    if let Some(err) = guard(pool, id, &string(args, 2), true).await? {
        return Ok(err);
    }
    let Some(level) = sqlx::query_scalar::<_, i32>("SELECT level FROM characters WHERE id=?")
        .bind(id)
        .fetch_optional(pool)
        .await?
    else {
        return Ok(error("Character not found"));
    };
    let Ok(code) = authentication::mission_token() else {
        return Ok(error("Entropy unavailable"));
    };
    let hash = authentication::sha256_hex(&format!("{zone_str}{id}{code}"));
    let rewards = hunting_zone_rewards(pool, zone_id).await?;
    let mut battles = state.hunting_battles.lock().await;
    battles.retain(|_, b| b.started.elapsed() < Duration::from_secs(1800));
    battles.insert(
        id,
        HuntingBattle {
            code: code.clone(),
            reward_xp: i64::from(level) * 100,
            reward_gold: i64::from(level) * 200,
            rewards,
            started: Instant::now(),
        },
    );
    Ok(json!({
        "status": 1,
        "error": 0,
        "code": code,
        "boss": format!("ene_{zone_str}"),
        "hash": hash
    }))
}

async fn hunting_finish(state: &GameState, pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let battle_code = string(args, 2);
    if let Some(err) = guard(pool, id, &string(args, 4), true).await? {
        return Ok(err);
    }
    let mut battles = state.hunting_battles.lock().await;
    let Some(battle) = battles.get(&id) else {
        return Ok(error("Invalid battle session"));
    };
    if battle.started.elapsed() >= Duration::from_secs(1800) || battle.code != battle_code {
        return Ok(error("Invalid battle session"));
    }

    let mut tx = pool.begin().await?;
    let (xp_reward, level, xp, level_up, tokens) =
        missions::award_battle_progress(&mut tx, id, battle.reward_xp, battle.reward_gold).await?;
    for reward in &battle.rewards {
        if let Some(item_id) = reward.as_str() {
            rewards::give_item(&mut tx, id, item_id, 1).await?;
        }
    }
    let reward_gold = battle.reward_gold;
    let reward_list = battle.rewards.clone();
    tx.commit().await?;
    battles.remove(&id);

    Ok(json!({
        "status": 1,
        "error": 0,
        "result": [xp_reward, reward_gold, reward_list],
        "level": level,
        "xp": xp,
        "level_up": level_up,
        "account_tokens": tokens
    }))
}

async fn load_hunting_recipes(
    pool: &MySqlPool,
) -> std::result::Result<Vec<(String, Vec<String>, Vec<i64>)>, sqlx::Error> {
    if let Some(config) = game_config(pool, "hunting_house_forge_recipes").await? {
        if let Some(obj) = config.as_object()
            && !obj.is_empty()
        {
            let mut out = Vec::with_capacity(obj.len());
            for (item, recipe) in obj {
                let mats = recipe["materials"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(Json::as_str)
                            .map(str::to_owned)
                            .collect()
                    })
                    .unwrap_or_default();
                let qtys = recipe["qty"]
                    .as_array()
                    .map(|a| a.iter().filter_map(Json::as_i64).collect())
                    .unwrap_or_default();
                out.push((item.clone(), mats, qtys));
            }
            return Ok(out);
        }
        if let Some(arr) = config.as_array()
            && !arr.is_empty()
        {
            let mut out = Vec::with_capacity(arr.len());
            for entry in arr {
                if let Some(item) = entry["item"].as_str() {
                    let mats = entry["materials"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(Json::as_str)
                                .map(str::to_owned)
                                .collect()
                        })
                        .unwrap_or_default();
                    let qtys = entry["qty"]
                        .as_array()
                        .map(|a| a.iter().filter_map(Json::as_i64).collect())
                        .unwrap_or_default();
                    out.push((item.to_owned(), mats, qtys));
                }
            }
            if !out.is_empty() {
                return Ok(out);
            }
        }
    }
    let catalog: Json = serde_json::from_str(include_str!(
        "../../../database/catalogs/hunting_forge.json"
    ))
    .expect("checked-in hunting_forge.json");
    let mut out = Vec::new();
    if let Some(arr) = catalog.as_array() {
        for entry in arr {
            if let Some(item) = entry["item"].as_str() {
                let mats = entry["materials"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(Json::as_str)
                            .map(str::to_owned)
                            .collect()
                    })
                    .unwrap_or_default();
                let qtys = entry["qty"]
                    .as_array()
                    .map(|a| a.iter().filter_map(Json::as_i64).collect())
                    .unwrap_or_default();
                out.push((item.to_owned(), mats, qtys));
            }
        }
    }
    Ok(out)
}

async fn hunting_get_items(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let recipes = load_hunting_recipes(pool).await?;
    let items: Vec<Json> = recipes
        .into_iter()
        .map(|(item, materials, qty)| {
            json!({
                "item": item,
                "requirements": {
                    "materials": materials,
                    "qty": qty
                }
            })
        })
        .collect();
    Ok(json!({
        "status": 1,
        "items": items
    }))
}

async fn hunting_forge_item(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let item_id = string(args, 2);
    let recipes = load_hunting_recipes(pool).await?;
    let Some((_, materials, qtys)) = recipes.into_iter().find(|(k, _, _)| *k == item_id) else {
        return Ok(json!({"status":2,"result":"Invalid item or recipe not found."}));
    };

    let mut tx = pool.begin().await?;
    for (idx, mat_id) in materials.iter().enumerate() {
        let required = qtys.get(idx).copied().unwrap_or(1).max(1);
        let held: i32 = sqlx::query_scalar(
            "SELECT quantity FROM character_items WHERE character_id=? AND item_id=? LIMIT 1 FOR UPDATE",
        )
        .bind(id)
        .bind(mat_id)
        .fetch_optional(&mut *tx)
        .await?
        .unwrap_or(0);
        if i64::from(held) < required {
            return Ok(json!({
                "status": 2,
                "result": format!("Not enough {mat_id} (have {held}, need {required}).")
            }));
        }
    }
    for (idx, mat_id) in materials.iter().enumerate() {
        let required = qtys.get(idx).copied().unwrap_or(1).max(1);
        if !inventory::take(&mut tx, id, mat_id, required).await? {
            return Ok(json!({
                "status": 2,
                "result": format!("Not enough {mat_id}.")
            }));
        }
    }
    rewards::give_item(&mut tx, id, &item_id, 1).await?;
    tx.commit().await?;

    Ok(json!({
        "status": 1,
        "item": item_id,
        "requirements": [materials, qtys]
    }))
}

// -----------------------------------------------------------------------------
// EudemonGarden.*
// -----------------------------------------------------------------------------

pub async fn dispatch_eudemon_garden(
    state: &GameState,
    pool: &MySqlPool,
    method: &str,
    args: &[Json],
) -> Result {
    match method {
        "getData" => eudemon_get_data(pool, args).await,
        "buyTries" => eudemon_buy_tries(pool, args).await,
        "startHunting" => eudemon_start(state, pool, args).await,
        "finishHunting" => eudemon_finish(state, pool, args).await,
        _ => Ok(error("Unknown EudemonGarden operation")),
    }
}

async fn eudemon_config(pool: &MySqlPool) -> std::result::Result<(Vec<Json>, i64), sqlx::Error> {
    let config = game_config(pool, "eudemon")
        .await?
        .unwrap_or_else(|| json!({}));
    let bosses = config["bosses"].as_array().cloned().unwrap_or_default();
    let default_tries = config["default_tries"].as_i64().unwrap_or(3);
    Ok((bosses, default_tries))
}

async fn eudemon_get_data(pool: &MySqlPool, args: &[Json]) -> Result {
    // Note: EudemonGarden.getData(sessionKey, charId) passes sessionKey FIRST!
    let key = string(args, 0);
    let id = integer(args, 1);
    if let Some(err) = guard(pool, id, &key, true).await? {
        return Ok(err);
    }
    let (bosses, default_tries) = eudemon_config(pool).await?;
    let boss_count = bosses.len();

    let mut tx = pool.begin().await?;
    let Some(row) = sqlx::query(
        "SELECT eudemon_garden_tries, CAST(eudemon_garden_date AS CHAR) AS eg_date, CAST(CURDATE() AS CHAR) AS today FROM characters WHERE id=? FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(json!({"status":0,"error_code":"Character not found"}));
    };
    let tries_opt: Option<String> = row.try_get("eudemon_garden_tries")?;
    let eg_date: Option<String> = row.try_get("eg_date")?;
    let today: String = row.try_get("today")?;

    let current_parts: Vec<&str> = tries_opt
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| s.split(',').collect())
        .unwrap_or_default();

    let tries_str = if eg_date.as_deref() != Some(today.as_str())
        || current_parts.len() != boss_count
    {
        let fresh = vec![default_tries.to_string(); boss_count].join(",");
        sqlx::query(
                "UPDATE characters SET eudemon_garden_date=CURDATE(), eudemon_garden_tries=?, updated_at=NOW() WHERE id=?",
            )
            .bind(&fresh)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        fresh
    } else {
        tries_opt.unwrap_or_default()
    };
    tx.commit().await?;

    Ok(json!({
        "status": 1,
        "data": tries_str
    }))
}

async fn eudemon_buy_tries(pool: &MySqlPool, args: &[Json]) -> Result {
    // Note: EudemonGarden.buyTries(sessionKey, charId) passes sessionKey FIRST!
    let key = string(args, 0);
    let id = integer(args, 1);
    if let Some(err) = guard(pool, id, &key, true).await? {
        return Ok(err);
    }
    let (bosses, default_tries) = eudemon_config(pool).await?;
    let boss_count = bosses.len();

    let mut tx = pool.begin().await?;
    let Some(row) = sqlx::query(
        "SELECT c.level, c.user_id, u.tokens FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(json!({"status":0,"error_code":"Character not found"}));
    };
    let level: i32 = row.try_get("level")?;
    let cost: i32 = if level >= 80 { 80 } else { 50 };
    let tokens: i32 = row.try_get("tokens")?;
    if tokens < cost {
        return Ok(json!({"status": 2}));
    }
    let user_id: u64 = row.try_get("user_id")?;
    sqlx::query("UPDATE users SET tokens=tokens-?,updated_at=NOW() WHERE id=?")
        .bind(cost)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    let fresh = vec![default_tries.to_string(); boss_count].join(",");
    sqlx::query("UPDATE characters SET eudemon_garden_tries=?,updated_at=NOW() WHERE id=?")
        .bind(&fresh)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(json!({
        "status": 1,
        "data": fresh
    }))
}

async fn eudemon_start(state: &GameState, pool: &MySqlPool, args: &[Json]) -> Result {
    // Params: [charId, bossNum, sessionKey]
    let id = integer(args, 0);
    let boss_num = integer(args, 1);
    let boss_str = string(args, 1);
    if let Some(err) = guard(pool, id, &string(args, 2), true).await? {
        return Ok(err);
    }
    let (bosses, _) = eudemon_config(pool).await?;
    let boss_idx = usize::try_from(boss_num).unwrap_or(usize::MAX);
    let Some(boss) = bosses.get(boss_idx) else {
        return Ok(error("Boss not found"));
    };

    let mut tx = pool.begin().await?;
    let Some(row) =
        sqlx::query("SELECT level, eudemon_garden_tries FROM characters WHERE id=? FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
    else {
        return Ok(error("Character not found"));
    };
    let level: i32 = row.try_get("level")?;
    let tries_raw: String = row
        .try_get::<Option<String>, _>("eudemon_garden_tries")?
        .unwrap_or_default();
    let mut tries: Vec<i64> = tries_raw
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<i64>().unwrap_or(0))
        .collect();
    if tries.get(boss_idx).copied().unwrap_or(0) <= 0 {
        return Ok(json!({"status":2,"result":"No tries left!"}));
    }
    let req_lvl = boss["lvl"].as_i64().unwrap_or(1);
    if i64::from(level) < req_lvl {
        return Ok(json!({"status":2,"result":"Level too low!"}));
    }
    let Ok(code) = authentication::mission_token() else {
        return Ok(error("Entropy unavailable"));
    };
    tries[boss_idx] -= 1;
    let updated_tries = tries
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    sqlx::query("UPDATE characters SET eudemon_garden_tries=?,updated_at=NOW() WHERE id=?")
        .bind(&updated_tries)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let hash = authentication::sha256_hex(&format!("{id}{code}{boss_str}"));
    let rewards = boss["rewards"].as_array().cloned().unwrap_or_default();
    let mut battles = state.eudemon_battles.lock().await;
    battles.retain(|_, b| b.started.elapsed() < Duration::from_secs(1800));
    battles.insert(
        id,
        HuntingBattle {
            code: code.clone(),
            reward_xp: boss["xp"].as_i64().unwrap_or(0),
            reward_gold: boss["gold"].as_i64().unwrap_or(0),
            rewards,
            started: Instant::now(),
        },
    );

    Ok(json!({
        "status": 1,
        "error": 0,
        "code": code,
        "boss": format!("boss_{boss_str}"),
        "hash": hash
    }))
}

async fn eudemon_finish(state: &GameState, pool: &MySqlPool, args: &[Json]) -> Result {
    // Params: [charId, bossNum, battleCode, hash, sessionKey, unknown]
    let id = integer(args, 0);
    let battle_code = string(args, 2);
    if let Some(err) = guard(pool, id, &string(args, 4), true).await? {
        return Ok(err);
    }
    let mut battles = state.eudemon_battles.lock().await;
    let Some(battle) = battles.get(&id) else {
        return Ok(error("Invalid battle session"));
    };
    if battle.started.elapsed() >= Duration::from_secs(1800) || battle.code != battle_code {
        return Ok(error("Invalid battle session"));
    }

    let mut tx = pool.begin().await?;
    let (xp_reward, level, xp, level_up, tokens) =
        missions::award_battle_progress(&mut tx, id, battle.reward_xp, battle.reward_gold).await?;

    let mut awarded_order: Vec<String> = Vec::new();
    let mut awarded_map: std::collections::HashMap<String, i64> = std::collections::HashMap::new();

    for reward_conf in &battle.rewards {
        if let Some(item_id) = reward_conf.as_str() {
            if !awarded_map.contains_key(item_id) {
                awarded_order.push(item_id.to_owned());
            }
            *awarded_map.entry(item_id.to_owned()).or_insert(0) += 1;
            continue;
        }
        let Some(item_id) = reward_conf["id"].as_str() else {
            continue;
        };
        let mut rate = reward_conf["rate"].as_f64().unwrap_or(100.0);
        let min = u32::try_from(reward_conf["min"].as_i64().unwrap_or(1).max(1)).unwrap_or(1);
        let max = u32::try_from(reward_conf["max"].as_i64().unwrap_or(1).max(1)).unwrap_or(1);
        let unique = reward_conf["unique"].as_bool().unwrap_or(false);
        let owned_rate = reward_conf["owned_rate"].as_f64().unwrap_or(0.0);

        if unique {
            let owned: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM character_items WHERE character_id=? AND item_id=?",
            )
            .bind(id)
            .bind(item_id)
            .fetch_one(&mut *tx)
            .await?;
            if owned > 0 {
                rate = owned_rate;
            }
        }
        let roll = f64::from(rand_inclusive(1, 10000));
        let threshold = rate * 100.0;
        if roll <= threshold {
            let qty = i64::from(rand_inclusive(min, max.max(min)));
            if !awarded_map.contains_key(item_id) {
                awarded_order.push(item_id.to_owned());
            }
            *awarded_map.entry(item_id.to_owned()).or_insert(0) += qty;
        }
    }

    let mut awarded_list = Vec::with_capacity(awarded_order.len());
    for item_id in awarded_order {
        let qty = awarded_map[&item_id];
        rewards::give_item(&mut tx, id, &item_id, qty).await?;
        if qty > 1 {
            awarded_list.push(format!("{item_id}:{qty}"));
        } else {
            awarded_list.push(item_id);
        }
    }

    let reward_gold = battle.reward_gold;
    tx.commit().await?;
    battles.remove(&id);

    Ok(json!({
        "status": 1,
        "error": 0,
        "result": [xp_reward, reward_gold, awarded_list],
        "level": level,
        "xp": xp,
        "level_up": level_up,
        "account_tokens": tokens
    }))
}
