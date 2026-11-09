//! Daily Reward, Daily Roulette, Daily Scratch, and Welcome Login AMF services.
//! Source:
//! - `app/Services/Amf/DailyRewardService/*`
//! - `app/Services/Amf/DailyRouletteService/*`
//! - `app/Services/Amf/DailyScratchService/*`
//! - `app/Services/Amf/WelcomeLoginService/*`
use crate::{
    account::{guard, integer, string},
    rewards,
};
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};

type Result = std::result::Result<Json, sqlx::Error>;

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
// DailyReward.*
// -----------------------------------------------------------------------------

pub async fn dispatch_daily_reward(pool: &MySqlPool, method: &str, args: &[Json]) -> Result {
    match method {
        "getDailyData" => daily_get_data(pool, args).await,
        "getDailyTokenData" => daily_get_token_data(pool, args).await,
        "claimDailyXP" => daily_claim_xp(pool, args).await,
        "claimDoubleXP" => daily_claim_double_xp(pool, args).await,
        "claimScrollOfWisdom" => daily_claim_scroll(pool, args).await,
        "getAttendances" => daily_get_attendances(pool, args).await,
        "claimAttendanceReward" => daily_claim_attendance(pool, args).await,
        _ => Ok(error("Unknown DailyReward operation")),
    }
}

async fn daily_get_data(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let Some(row) = sqlx::query(
        "SELECT IF(daily_token_claimed_at IS NULL OR daily_token_claimed_at < CURDATE(), 1, 0) AS tokens_avail, IF(daily_xp_claimed_at IS NULL OR daily_xp_claimed_at < CURDATE(), 1, 0) AS xp_avail, IF(daily_scroll_claimed_at IS NOT NULL, 1, 0) AS scroll_claimed, IF(double_xp_expire_at IS NOT NULL AND double_xp_expire_at > NOW(), TIMESTAMPDIFF(SECOND, NOW(), double_xp_expire_at), 0) AS timer FROM characters WHERE id=?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(error("Character not found"));
    };

    Ok(json!({
        "status": 1,
        "tokens": row.try_get::<i32, _>("tokens_avail")? != 0,
        "xp": row.try_get::<i32, _>("xp_avail")? != 0,
        "scroll": row.try_get::<i32, _>("scroll_claimed")? != 0,
        "timer": row.try_get::<i64, _>("timer")?.max(0)
    }))
}

async fn daily_get_token_data(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let config = game_config(pool, "daily_rewards")
        .await?
        .unwrap_or_else(|| json!({}));
    let token_amount = config["token_amount"].as_i64().unwrap_or(25);

    let mut tx = pool.begin().await?;
    let Some(row) = sqlx::query(
        "SELECT user_id, IF(daily_token_claimed_at IS NOT NULL AND daily_token_claimed_at >= CURDATE(), 1, 0) AS claimed FROM characters WHERE id=? FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(error("Character not found"));
    };
    if row.try_get::<i32, _>("claimed")? != 0 {
        return Ok(json!({"status":2,"result":"Already claimed today!"}));
    }
    let user_id: u64 = row.try_get("user_id")?;
    sqlx::query("UPDATE users SET tokens=tokens+?,updated_at=NOW() WHERE id=?")
        .bind(token_amount)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE characters SET daily_token_claimed_at=NOW(),updated_at=NOW() WHERE id=?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(json!({"status": 1}))
}

async fn daily_claim_xp(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let config = game_config(pool, "daily_rewards")
        .await?
        .unwrap_or_else(|| json!({}));
    let double_xp_chance =
        u32::try_from(config["double_xp_chance"].as_i64().unwrap_or(20).max(0)).unwrap_or(20);
    let min_mult =
        u32::try_from(config["xp_min_multiplier"].as_i64().unwrap_or(50).max(1)).unwrap_or(50);
    let max_mult =
        u32::try_from(config["xp_max_multiplier"].as_i64().unwrap_or(100).max(1)).unwrap_or(100);

    let mut tx = pool.begin().await?;
    let Some(row) = sqlx::query(
        "SELECT level, `rank`, xp, IF(daily_xp_claimed_at IS NOT NULL AND daily_xp_claimed_at >= CURDATE(), 1, 0) AS claimed FROM characters WHERE id=? FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(error("Character not found"));
    };
    if row.try_get::<i32, _>("claimed")? != 0 {
        return Ok(json!({"status":2,"result":"Already claimed today!"}));
    }
    let level: i32 = row.try_get("level")?;
    let rank: i32 = row.try_get("rank")?;
    let current_xp: i64 = row.try_get("xp")?;

    let roll = rand_inclusive(1, 100);
    let mut xp_reward: i64 = 0;
    let mut double_xp_duration: i64 = 0;
    let mut level_up = false;
    let mut bonus_rate: i32 = 0;
    let mut final_xp = current_xp;

    if roll <= double_xp_chance {
        double_xp_duration = 3600;
        bonus_rate = i32::try_from(rand_inclusive(10, 30)).unwrap_or(20);
        sqlx::query(
            "UPDATE characters SET double_xp_expire_at=DATE_ADD(NOW(), INTERVAL 3600 SECOND), xp_bonus_rate=?, daily_xp_claimed_at=NOW(), updated_at=NOW() WHERE id=?",
        )
        .bind(bonus_rate)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    } else {
        let mult = i64::from(rand_inclusive(min_mult, max_mult.max(min_mult)));
        xp_reward = if level >= rewards::rank_cap(rank) {
            0
        } else {
            i64::from(level) * mult
        };
        let (_, new_xp, lvl_up) = rewards::add_xp(&mut tx, id, xp_reward).await?;
        final_xp = new_xp;
        level_up = lvl_up;
        sqlx::query(
            "UPDATE characters SET xp_bonus_rate=0, double_xp_expire_at=NULL, daily_xp_claimed_at=NOW(), updated_at=NOW() WHERE id=?",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok(json!({
        "status": 1,
        "xp": final_xp,
        "reward": xp_reward,
        "level_up": level_up,
        "double_xp": double_xp_duration > 0,
        "timer": double_xp_duration,
        "bonus_rate": bonus_rate
    }))
}

async fn daily_claim_double_xp(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let mut tx = pool.begin().await?;
    let Some(row) = sqlx::query(
        "SELECT IF(daily_xp_claimed_at IS NOT NULL AND daily_xp_claimed_at >= CURDATE(), 1, 0) AS claimed FROM characters WHERE id=? FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(error("Character not found"));
    };
    if row.try_get::<i32, _>("claimed")? != 0 {
        return Ok(json!({"status":2,"result":"Already claimed today!"}));
    }
    let duration = 3600;
    sqlx::query(
        "UPDATE characters SET double_xp_expire_at=DATE_ADD(NOW(), INTERVAL 3600 SECOND), daily_xp_claimed_at=NOW(), updated_at=NOW() WHERE id=?",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(json!({
        "status": 1,
        "timer": duration
    }))
}

async fn daily_claim_scroll(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let mut tx = pool.begin().await?;
    let Some(row) = sqlx::query(
        "SELECT level, IF(daily_scroll_claimed_at IS NOT NULL, 1, 0) AS claimed FROM characters WHERE id=? FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(error("Character not found"));
    };
    let level: i32 = row.try_get("level")?;
    if level < 80 {
        return Ok(json!({"status":2,"result":"You must be Level 80 to claim this reward!"}));
    }
    if row.try_get::<i32, _>("claimed")? != 0 {
        return Ok(json!({"status":2,"result":"Already claimed!"}));
    }
    rewards::give_item(&mut tx, id, "essential_10", 1).await?;
    sqlx::query("UPDATE characters SET daily_scroll_claimed_at=NOW(),updated_at=NOW() WHERE id=?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(json!({"status": 1}))
}

async fn daily_get_attendances(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let config_rewards = game_config(pool, "attendance_rewards")
        .await?
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();

    let mut tx = pool.begin().await?;
    let Some(row) = sqlx::query(
        "SELECT CAST(attendance_days AS CHAR) AS days_json, CAST(attendance_rewards AS CHAR) AS rewards_json, DATE_FORMAT(attendance_last_reset, '%Y-%m') AS last_month, DATE_FORMAT(CURDATE(), '%Y-%m') AS this_month, DAYOFMONTH(CURDATE()) AS today_day FROM characters WHERE id=? FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(error("Character not found"));
    };

    let last_month: Option<String> = row.try_get("last_month")?;
    let this_month: String = row.try_get("this_month")?;
    let today_day: i32 = row.try_get("today_day")?;

    let (mut days, claim_statuses) = if last_month.as_deref() != Some(this_month.as_str()) {
        (Vec::<i32>::new(), vec![0, 0, 0, 0, 0, 0])
    } else {
        let d = row
            .try_get::<Option<String>, _>("days_json")?
            .and_then(|s| serde_json::from_str::<Vec<i32>>(&s).ok())
            .unwrap_or_default();
        let r = row
            .try_get::<Option<String>, _>("rewards_json")?
            .and_then(|s| serde_json::from_str::<Vec<i32>>(&s).ok())
            .unwrap_or_else(|| vec![0, 0, 0, 0, 0, 0]);
        (d, r)
    };

    if !days.contains(&today_day) {
        days.push(today_day);
        days.sort_unstable();
    }

    let days_str = serde_json::to_string(&days).unwrap_or_else(|_| "[]".into());
    let rewards_str =
        serde_json::to_string(&claim_statuses).unwrap_or_else(|_| "[0,0,0,0,0,0]".into());

    if last_month.as_deref() != Some(this_month.as_str()) {
        sqlx::query(
            "UPDATE characters SET attendance_days=CAST(? AS JSON), attendance_rewards=CAST(? AS JSON), attendance_last_reset=CURDATE(), updated_at=NOW() WHERE id=?",
        )
        .bind(&days_str)
        .bind(&rewards_str)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query(
            "UPDATE characters SET attendance_days=CAST(? AS JSON), updated_at=NOW() WHERE id=?",
        )
        .bind(&days_str)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok(json!({
        "status": 1,
        "count": days.len(),
        "attendances": days,
        "rewards": claim_statuses,
        "items": config_rewards
    }))
}

async fn daily_claim_attendance(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let reward_id = string(args, 2);
    let config_rewards = game_config(pool, "attendance_rewards")
        .await?
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();

    let Some((index, reward_entry)) = config_rewards
        .iter()
        .enumerate()
        .find(|(_, r)| string(&[r["id"].clone()], 0) == reward_id)
    else {
        return Ok(json!({"status":2,"result":"Invalid Reward"}));
    };

    let price = usize::try_from(reward_entry["price"].as_i64().unwrap_or(0).max(0)).unwrap_or(0);
    let reward_item = reward_entry["item"].as_str().unwrap_or("");

    let mut tx = pool.begin().await?;
    let Some(row) = sqlx::query(
        "SELECT CAST(attendance_days AS CHAR) AS days_json, CAST(attendance_rewards AS CHAR) AS rewards_json FROM characters WHERE id=? FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(error("Character not found"));
    };

    let days: Vec<i32> = row
        .try_get::<Option<String>, _>("days_json")?
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let mut claim_statuses: Vec<i32> = row
        .try_get::<Option<String>, _>("rewards_json")?
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| vec![0, 0, 0, 0, 0, 0]);
    if claim_statuses.len() <= index {
        claim_statuses.resize(index + 1, 0);
    }

    if days.len() < price {
        return Ok(json!({"status":2,"result":"Not enough attendance days!"}));
    }
    if claim_statuses[index] == 1 {
        return Ok(json!({"status":2,"result":"Already claimed!"}));
    }

    let level_up = rewards::grant(&mut tx, id, reward_item).await?;
    claim_statuses[index] = 1;
    let rewards_str = serde_json::to_string(&claim_statuses).unwrap_or_else(|_| "[]".into());
    sqlx::query(
        "UPDATE characters SET attendance_rewards=CAST(? AS JSON), updated_at=NOW() WHERE id=?",
    )
    .bind(rewards_str)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    let xp: i64 = sqlx::query_scalar("SELECT xp FROM characters WHERE id=?")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(json!({
        "status": 1,
        "reward": reward_item,
        "rewards": claim_statuses,
        "xp": xp,
        "level_up": level_up
    }))
}

// -----------------------------------------------------------------------------
// DailyRoulette.*
// -----------------------------------------------------------------------------

pub async fn dispatch_daily_roulette(pool: &MySqlPool, method: &str, args: &[Json]) -> Result {
    match method {
        "getData" => roulette_get_data(pool, args).await,
        "spin" => roulette_spin(pool, args).await,
        _ => Ok(error("Unknown DailyRoulette operation")),
    }
}

async fn roulette_get_data(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let mut tx = pool.begin().await?;
    let Some(row) = sqlx::query(
        "SELECT c.daily_roulette_consecutive, c.daily_roulette_count, DATEDIFF(CURDATE(), c.daily_roulette_date) AS days_diff, u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(error("Character not found"));
    };

    let days_diff: Option<i32> = row.try_get("days_diff")?;
    let mut consecutive: i32 = row.try_get("daily_roulette_consecutive")?;
    let mut count: i32 = row.try_get("daily_roulette_count")?;

    if days_diff.is_none() || days_diff.unwrap_or(0) > 0 {
        if days_diff == Some(1) {
            consecutive += 1;
            if consecutive > 7 {
                consecutive = 1;
            }
        } else {
            consecutive = 1;
        }
        count = 0;
        sqlx::query(
            "UPDATE characters SET daily_roulette_consecutive=?, daily_roulette_count=0, daily_roulette_date=CURDATE(), updated_at=NOW() WHERE id=?",
        )
        .bind(consecutive)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }
    let account_type: i8 = row.try_get("account_type")?;
    let max_spins = if account_type == 1 { 2 } else { 1 };
    let can_spin = if count < max_spins { 1 } else { 0 };
    tx.commit().await?;

    Ok(json!({
        "status": 1,
        "can_spin": can_spin,
        "bonus": consecutive
    }))
}

async fn roulette_spin(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let rewards_cfg = game_config(pool, "roulette_rewards")
        .await?
        .unwrap_or_else(|| json!({"1": "gold_1000"}));

    let mut frame_list: Vec<(Json, String)> = Vec::new();
    if let Some(obj) = rewards_cfg.as_object() {
        for (k, v) in obj {
            if let Some(s) = v.as_str() {
                let key_val = k.parse::<i64>().map_or_else(|_| json!(k), |n| json!(n));
                frame_list.push((key_val, s.to_owned()));
            }
        }
    } else if let Some(arr) = rewards_cfg.as_array() {
        for (idx, v) in arr.iter().enumerate() {
            if let Some(s) = v.as_str() {
                frame_list.push((json!(idx), s.to_owned()));
            }
        }
    }
    if frame_list.is_empty() {
        frame_list.push((json!(1), "gold_1000".into()));
    }

    let mut tx = pool.begin().await?;
    let Some(row) = sqlx::query(
        "SELECT c.user_id, c.level, c.`rank`, c.xp, c.gold, c.daily_roulette_consecutive, c.daily_roulette_count, DATEDIFF(CURDATE(), c.daily_roulette_date) AS days_diff, u.account_type, u.tokens FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(error("Character not found"));
    };

    let days_diff: Option<i32> = row.try_get("days_diff")?;
    let consecutive: i32 = row.try_get("daily_roulette_consecutive")?;
    let mut count: i32 = row.try_get("daily_roulette_count")?;
    if days_diff.is_none() || days_diff.unwrap_or(0) > 0 {
        count = 0;
        sqlx::query(
            "UPDATE characters SET daily_roulette_date=CURDATE(), daily_roulette_count=0, updated_at=NOW() WHERE id=?",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }
    let account_type: i8 = row.try_get("account_type")?;
    let max_spins = if account_type == 1 { 2 } else { 1 };
    if count >= max_spins {
        return Ok(json!({"status": 2}));
    }

    let pick_idx = usize::try_from(rand_inclusive(
        0,
        u32::try_from(frame_list.len() - 1).unwrap_or(0),
    ))
    .unwrap_or(0);
    let (reward_frame, base_reward) = &frame_list[pick_idx];

    let multiplier = i64::from(consecutive.max(1));
    let mut parts = base_reward.splitn(2, '_');
    let rtype = parts.next().unwrap_or("gold");
    let rval = parts
        .next()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0)
        * multiplier;
    let final_reward_str = format!("{rtype}_{rval}");

    let user_id: u64 = row.try_get("user_id")?;
    let mut xp: i64 = row.try_get("xp")?;
    let mut gold: i32 = row.try_get("gold")?;
    let mut tokens: i32 = row.try_get("tokens")?;
    let mut level_up = false;

    match rtype {
        "gold" => {
            let new_gold = i64::from(gold)
                .saturating_add(rval)
                .min(i64::from(i32::MAX));
            gold = i32::try_from(new_gold).unwrap_or(i32::MAX);
            sqlx::query("UPDATE characters SET gold=?,updated_at=NOW() WHERE id=?")
                .bind(gold)
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
        "tokens" => {
            let new_tokens = i64::from(tokens)
                .saturating_add(rval)
                .min(i64::from(i32::MAX));
            tokens = i32::try_from(new_tokens).unwrap_or(i32::MAX);
            sqlx::query("UPDATE users SET tokens=?,updated_at=NOW() WHERE id=?")
                .bind(tokens)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
        }
        "xp" => {
            let level: i32 = row.try_get("level")?;
            let rank: i32 = row.try_get("rank")?;
            let xp_to_add = if level >= rewards::rank_cap(rank) {
                0
            } else {
                rval
            };
            let (_, new_xp, lvl_up) = rewards::add_xp(&mut tx, id, xp_to_add).await?;
            xp = new_xp;
            level_up = lvl_up;
        }
        _ => {}
    }

    sqlx::query(
        "UPDATE characters SET daily_roulette_count=daily_roulette_count+1, updated_at=NOW() WHERE id=?",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(json!({
        "status": 1,
        "reward_string": final_reward_str,
        "xp": xp,
        "tokens": tokens,
        "gold": gold,
        "reward": reward_frame,
        "bonus": consecutive,
        "level_up": level_up
    }))
}

// -----------------------------------------------------------------------------
// DailyScratch.*
// -----------------------------------------------------------------------------

pub async fn dispatch_daily_scratch(pool: &MySqlPool, method: &str, args: &[Json]) -> Result {
    match method {
        "getData" => scratch_get_data(pool, args).await,
        "scratch" => scratch_execute(pool, args).await,
        _ => Ok(error("Unknown DailyScratch operation")),
    }
}

async fn scratch_get_data(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let mut tx = pool.begin().await?;
    let Some(row) = sqlx::query(
        "SELECT c.daily_scratch_consecutive, c.daily_scratch_count, DATEDIFF(CURDATE(), c.daily_scratch_date) AS days_diff, u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(error("Character not found"));
    };

    let days_diff: Option<i32> = row.try_get("days_diff")?;
    let mut consecutive: i32 = row.try_get("daily_scratch_consecutive")?;
    let mut count: i32 = row.try_get("daily_scratch_count")?;

    if days_diff.is_none() || days_diff.unwrap_or(0) > 0 {
        if days_diff == Some(1) {
            consecutive += 1;
        } else {
            consecutive = 1;
        }
        count = 0;
        sqlx::query(
            "UPDATE characters SET daily_scratch_consecutive=?, daily_scratch_count=0, daily_scratch_date=CURDATE(), updated_at=NOW() WHERE id=?",
        )
        .bind(consecutive)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }
    let account_type: i8 = row.try_get("account_type")?;
    let max_tickets = if account_type == 1 {
        1 + consecutive
    } else {
        1
    };
    let tickets = (max_tickets - count).max(0);
    tx.commit().await?;

    Ok(json!({
        "status": 1,
        "consecutive": consecutive,
        "ticket": tickets
    }))
}

async fn scratch_rewards_config(
    pool: &MySqlPool,
) -> std::result::Result<(Vec<String>, Vec<String>), sqlx::Error> {
    if let Some(cfg) = game_config(pool, "scratch").await? {
        if let Some(arr) = cfg["rewards"].as_array() {
            let rewards = arr
                .iter()
                .filter_map(Json::as_str)
                .map(str::to_owned)
                .collect();
            let grand = cfg["grand_prize"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(Json::as_str)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default();
            return Ok((rewards, grand));
        }
        if let Some(arr) = cfg.as_array() {
            let rewards = arr
                .iter()
                .filter_map(Json::as_str)
                .map(str::to_owned)
                .collect();
            return Ok((rewards, Vec::new()));
        }
    }
    if let Some(legacy) = game_config(pool, "scratch_rewards").await?
        && let Some(arr) = legacy.as_array()
    {
        let rewards = arr
            .iter()
            .filter_map(Json::as_str)
            .map(str::to_owned)
            .collect();
        return Ok((rewards, Vec::new()));
    }
    Ok((Vec::new(), Vec::new()))
}

fn roll_chance(chance: f64) -> bool {
    let clamped = chance.clamp(0.0, 1.0);
    let threshold = (clamped * 10000.0).round() as u32;
    rand_inclusive(1, 10000) <= threshold
}

fn pick_random_str(list: &[String]) -> Option<String> {
    if list.is_empty() {
        return None;
    }
    let idx = usize::try_from(rand_inclusive(
        0,
        u32::try_from(list.len() - 1).unwrap_or(0),
    ))
    .unwrap_or(0);
    list.get(idx).cloned()
}

async fn scratch_execute(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let (rewards_pool, grand_pool) = scratch_rewards_config(pool).await?;
    if rewards_pool.is_empty() {
        return Ok(error("Invalid scratch reward config"));
    }

    let mut tx = pool.begin().await?;
    let Some(row) = sqlx::query(
        "SELECT c.daily_scratch_consecutive, c.daily_scratch_count, c.scratch_grand_progress, c.scratch_rare_progress, u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(error("Character not found"));
    };

    let consecutive: i32 = row.try_get("daily_scratch_consecutive")?;
    let count: i32 = row.try_get("daily_scratch_count")?;
    let account_type: i8 = row.try_get("account_type")?;
    let max_tickets = if account_type == 1 {
        1 + consecutive
    } else {
        1
    };
    if count >= max_tickets {
        return Ok(json!({"status":2,"result":"No tickets left!"}));
    }

    let owned_pets: std::collections::HashSet<String> =
        sqlx::query("SELECT pet_id FROM character_pets WHERE character_id=?")
            .bind(id)
            .fetch_all(&mut *tx)
            .await?
            .iter()
            .filter_map(|r| r.try_get::<String, _>("pet_id").ok())
            .collect();
    let owned_skills: std::collections::HashSet<String> =
        sqlx::query("SELECT skill_id FROM character_skills WHERE character_id=?")
            .bind(id)
            .fetch_all(&mut *tx)
            .await?
            .iter()
            .filter_map(|r| r.try_get::<String, _>("skill_id").ok())
            .collect();

    let is_unowned = |r: &String| -> bool {
        if r.starts_with("pet_") && owned_pets.contains(r) {
            return false;
        }
        if r.starts_with("skill_") && owned_skills.contains(r) {
            return false;
        }
        true
    };

    let eligible: Vec<String> = rewards_pool
        .iter()
        .filter(|r| is_unowned(r))
        .cloned()
        .collect();
    let rare: Vec<String> = eligible
        .iter()
        .filter(|r| r.starts_with("pet_") || r.starts_with("skill_"))
        .cloned()
        .collect();
    let common: Vec<String> = eligible
        .iter()
        .filter(|r| !r.starts_with("pet_") && !r.starts_with("skill_"))
        .cloned()
        .collect();

    let mut grand_progress = row.try_get::<u32, _>("scratch_grand_progress")? + 1;
    let mut rare_progress = row.try_get::<u32, _>("scratch_rare_progress")? + 1;

    let grant_grand = !grand_pool.is_empty()
        && grand_progress >= 100
        && (grand_progress >= 500 || roll_chance(f64::from(grand_progress - 100) / 400.0));
    let grant_rare = !rare.is_empty()
        && rare_progress >= 50
        && (rare_progress >= 200 || roll_chance(f64::from(rare_progress - 50) / 150.0));

    let chosen = if grant_grand {
        grand_progress = 0;
        pick_random_str(&grand_pool)
    } else if grant_rare {
        rare_progress = 0;
        pick_random_str(&rare)
    } else if !common.is_empty() {
        pick_random_str(&common)
    } else {
        pick_random_str(&eligible)
    };

    let Some(reward_str) = chosen else {
        return Ok(error("Invalid scratch reward config"));
    };

    rewards::grant(&mut tx, id, &reward_str).await?;
    sqlx::query(
        "UPDATE characters SET daily_scratch_count=daily_scratch_count+1, scratch_grand_progress=?, scratch_rare_progress=?, updated_at=NOW() WHERE id=?",
    )
    .bind(grand_progress)
    .bind(rare_progress)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(json!({
        "status": 1,
        "reward": reward_str
    }))
}

// -----------------------------------------------------------------------------
// WelcomeLogin.*
// -----------------------------------------------------------------------------

const WELCOME_DEFAULTS: [&str; 7] = [
    "gold_250000",
    "item_45:5",
    "essential_01:1",
    "essential_05:5",
    "essential_12:1",
    "tokens_500",
    "skill_2158",
];

pub async fn dispatch_welcome_login(pool: &MySqlPool, method: &str, args: &[Json]) -> Result {
    match method {
        "get" => welcome_login_get(pool, args).await,
        "claim" => welcome_login_claim(pool, args).await,
        _ => Ok(error("Unknown WelcomeLogin operation")),
    }
}

async fn welcome_rewards_catalog(
    pool: &MySqlPool,
) -> std::result::Result<Vec<String>, sqlx::Error> {
    let cfg = game_config(pool, "welcome_login_rewards").await?;
    let mut list: Vec<String> = cfg
        .and_then(|v| v.as_array().cloned())
        .map(|a| {
            a.iter()
                .map(|x| x.as_str().unwrap_or("").to_owned())
                .collect()
        })
        .unwrap_or_default();
    for (i, default) in WELCOME_DEFAULTS.iter().enumerate() {
        if i >= list.len() {
            list.push((*default).to_owned());
        } else if list[i].is_empty() {
            list[i] = (*default).to_owned();
        }
    }
    list.truncate(7);
    Ok(list)
}

async fn welcome_login_get(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let Some(row) = sqlx::query(
        "SELECT claimed_welcome_rewards, GREATEST(1, DATEDIFF(CURDATE(), DATE(created_at)) + 1) AS login_days FROM characters WHERE id=?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(error("Character not found"));
    };

    let login_days: i64 = row.try_get("login_days")?;
    let claimed_raw: Option<String> = row.try_get("claimed_welcome_rewards")?;
    let claimed_set: std::collections::HashSet<String> = claimed_raw
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| s.split(',').map(|p| p.trim().to_owned()).collect())
        .unwrap_or_default();

    let catalog = welcome_rewards_catalog(pool).await?;
    let rewards_out: Vec<Json> = catalog
        .into_iter()
        .enumerate()
        .map(|(index, reward_str)| {
            let is_claimed = i32::from(claimed_set.contains(&index.to_string()));
            json!({
                "r": reward_str,
                "c": is_claimed
            })
        })
        .collect();

    Ok(json!({
        "status": 1,
        "error": 0,
        "logins": login_days.to_string(),
        "rewards": rewards_out
    }))
}

async fn welcome_login_claim(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let reward_idx_str = string(args, 2);
    let reward_idx = usize::try_from(integer(args, 2).max(0)).unwrap_or(usize::MAX);
    let catalog = welcome_rewards_catalog(pool).await?;
    let reward_str = catalog.get(reward_idx).cloned().unwrap_or_default();

    let mut tx = pool.begin().await?;
    let Some(row) =
        sqlx::query("SELECT claimed_welcome_rewards FROM characters WHERE id=? FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
    else {
        return Ok(error("Character not found"));
    };

    let claimed_raw: Option<String> = row.try_get("claimed_welcome_rewards")?;
    let mut claimed_list: Vec<String> = claimed_raw
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| s.split(',').map(|p| p.trim().to_owned()).collect())
        .unwrap_or_default();

    if !claimed_list.contains(&reward_idx_str) {
        claimed_list.push(reward_idx_str);
        let updated = claimed_list.join(",");
        if !reward_str.is_empty() {
            rewards::grant(&mut tx, id, &reward_str).await?;
        }
        sqlx::query("UPDATE characters SET claimed_welcome_rewards=?, updated_at=NOW() WHERE id=?")
            .bind(updated)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;

    Ok(json!({
        "status": 1,
        "error": 0,
        "rewards": reward_str
    }))
}
