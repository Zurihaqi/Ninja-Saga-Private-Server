use crate::{
    GameState,
    account::{guard, integer, string},
};
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};
use std::time::{Duration, Instant};
pub struct Battle {
    token: String,
    mission: String,
    xp: i64,
    gold: i64,
    started: Instant,
}
type Result = std::result::Result<Json, sqlx::Error>;
fn error(message: &str) -> Json {
    json!({"status":0,"error":message})
}

pub async fn s_data(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(e) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(e);
    }
    let mut materials = json!({"material_899":0,"material_900":0});
    let rows=sqlx::query("SELECT item_id,CAST(SUM(quantity) AS SIGNED) AS quantity FROM character_items WHERE character_id=? AND item_id IN ('material_899','material_900') GROUP BY item_id").bind(id).fetch_all(pool).await?;
    for row in rows {
        materials[row.try_get::<String, _>("item_id")?] = json!(row.try_get::<i64, _>("quantity")?);
    }
    // The reference exposes the initial S-mission state; its start/finish path
    // never changes these cache values. Keep that contract until staged S battles exist.
    Ok(json!({"status":1,"error":0,"stage":1,"energy":25,"max_energy":25,"materials":materials}))
}

pub async fn start(state: &GameState, pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let requested = string(args, 1);
    if let Some(error) = guard(pool, id, &string(args, 6), true).await? {
        return Ok(error);
    }
    let canonical = requested
        .strip_prefix("msn_")
        .and_then(|s| s.parse::<u32>().ok())
        .map(|n| format!("msn_{n:02}"))
        .unwrap_or_else(|| requested.clone());
    let mission=sqlx::query("SELECT mission_id,req_lvl,xp,gold FROM missions WHERE mission_id IN (?,?) ORDER BY mission_id LIMIT 1").bind(&requested).bind(&canonical).fetch_optional(pool).await?;
    let Some(mission) = mission else {
        return Ok(error("Mission data not found"));
    };
    let level: i32 = sqlx::query_scalar("SELECT level FROM characters WHERE id=?")
        .bind(id)
        .fetch_one(pool)
        .await?;
    if level < mission.try_get::<i32, _>("req_lvl")? {
        return Ok(error("Level too low"));
    }
    let path = std::env::var("GAME_DATA_ROOT")
        .ok()
        .map(|root| std::path::Path::new(&root).join("mission.json"));
    let definitions = path
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| serde_json::from_slice::<Json>(&b).ok());
    let mission_id: String = mission.try_get("mission_id")?;
    let definition = definitions
        .as_ref()
        .and_then(Json::as_array)
        .and_then(|list| list.iter().find(|v| v["id"] == mission_id));
    let Some(definition) = definition else {
        return Ok(error("Mission data not found"));
    };
    let expected: Vec<_> = definition["enemies"]
        .as_array()
        .map(|v| v.iter().filter_map(Json::as_str).collect())
        .unwrap_or_default();
    let enemies = string(args, 2);
    let stats = string(args, 3);
    if !expected.is_empty() {
        if expected
            != enemies
                .split(',')
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        {
            return Ok(error("Invalid enemies"));
        }
        for enemy in expected {
            let found = stats.split('#').find_map(|chunk| {
                let map: std::collections::HashMap<_, _> =
                    chunk.split('|').filter_map(|s| s.split_once(':')).collect();
                if map.get("id") == Some(&enemy) {
                    Some((
                        map.get("hp").and_then(|s| s.parse::<i64>().ok()),
                        map.get("agility").and_then(|s| s.parse::<i64>().ok()),
                    ))
                } else {
                    None
                }
            });
            let Some((hp, agility)) = found else {
                return Ok(error("Invalid enemy stats"));
            };
            let Some(row) = sqlx::query("SELECT hp,agility FROM enemies WHERE enemy_id=?")
                .bind(enemy)
                .fetch_optional(pool)
                .await?
            else {
                return Ok(error("Invalid enemy"));
            };
            if hp != Some(i64::from(row.try_get::<i32, _>("hp")?))
                || agility != Some(i64::from(row.try_get::<i32, _>("agility")?))
            {
                return Ok(error("Invalid enemy stats"));
            }
        }
        if !authentication::valid_legacy_hash(
            &string(args, 5),
            &format!("{enemies}{stats}{}", string(args, 4)),
        ) {
            return Ok(error("Invalid mission data"));
        }
    }
    let Ok(token) = authentication::mission_token() else {
        return Ok(error("Entropy unavailable"));
    };
    let mut battles = state.battles.lock().await;
    battles.retain(|_, b| b.started.elapsed() < Duration::from_secs(1800));
    if battles.len() >= 4096 && !battles.contains_key(&id) {
        return Ok(error("Server battle capacity reached"));
    }
    battles.insert(
        id,
        Battle {
            token: token.clone(),
            mission: mission_id,
            xp: mission.try_get::<i32, _>("xp")?.into(),
            gold: mission.try_get::<i32, _>("gold")?.into(),
            started: Instant::now(),
        },
    );
    Ok(json!(token))
}

pub async fn finish(state: &GameState, pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let mission = string(args, 1);
    let token = string(args, 2);
    if let Some(error) = guard(pool, id, &string(args, 5), true).await? {
        return Ok(error);
    }
    // A token is consumed only after the reward transaction commits. Holding the
    // async lock serializes duplicate completion requests, including two HTTP calls.
    let mut battles = state.battles.lock().await;
    let Some(battle) = battles.get(&id) else {
        return Ok(error("Invalid battle token"));
    };
    if battle.started.elapsed() >= Duration::from_secs(1800)
        || battle.token != token
        || battle.mission != mission
    {
        return Ok(error("Invalid battle token"));
    }
    if !authentication::valid_legacy_hash(
        &string(args, 3),
        &format!("{mission}{id}{token}{}", string(args, 4)),
    ) {
        return Ok(error("Invalid battle data"));
    }
    let mut tx = pool.begin().await?;
    let (xp_reward, level, xp, level_up, tokens) =
        award_battle_progress(&mut tx, id, battle.xp, battle.gold).await?;
    sqlx::query("DELETE FROM character_recruits WHERE character_id=?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let result = json!({"status":1,"error":0,"result":[xp_reward,battle.gold,[]],"level":level,"xp":xp,"level_up":level_up,"account_tokens":tokens});
    tx.commit().await?;
    battles.remove(&id);
    Ok(result)
}

pub(crate) async fn award_battle_progress(
    tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
    id: i64,
    base_xp: i64,
    gold_reward: i64,
) -> std::result::Result<(i64, i32, i64, bool, i32), sqlx::Error> {
    let row = sqlx::query("SELECT gold,xp,level,`rank`,user_id,equipment_pet,IF(double_xp_expire_at>NOW(),xp_bonus_rate,0) AS bonus FROM characters WHERE id=? FOR UPDATE")
        .bind(id)
        .fetch_one(&mut **tx)
        .await?;
    let old_level: i32 = row.try_get("level")?;
    let rank: i32 = row.try_get("rank")?;
    let mut level = old_level;
    let cap = match rank {
        1 => 20,
        3 => 40,
        5 => 60,
        7 => 80,
        _ => 85,
    };
    let bonus: i32 = row.try_get("bonus")?;
    let xp_reward = if bonus > 0 {
        base_xp * (100 + i64::from(bonus)) / 100
    } else {
        base_xp
    };
    let mut xp = row
        .try_get::<i64, _>("xp")?
        .checked_add(xp_reward)
        .ok_or_else(|| sqlx::Error::Protocol("XP overflow".into()))?;
    let gold = i64::from(row.try_get::<i32, _>("gold")?)
        .checked_add(gold_reward)
        .filter(|v| *v <= i64::from(i32::MAX))
        .ok_or_else(|| sqlx::Error::Protocol("Gold overflow".into()))?;
    while level < cap {
        let required: Option<i64> =
            sqlx::query_scalar("SELECT character_xp FROM x_p_s WHERE level=?")
                .bind(level)
                .fetch_optional(&mut **tx)
                .await?;
        if xp >= required.filter(|v| *v > 0).unwrap_or(999999999) {
            level += 1;
        } else {
            break;
        }
    }
    if level >= cap
        && let Some(required) =
            sqlx::query_scalar::<_, i64>("SELECT character_xp FROM x_p_s WHERE level=?")
                .bind(level)
                .fetch_optional(&mut **tx)
                .await?
        && required > 0
    {
        xp = xp.min(required);
    }
    sqlx::query("UPDATE characters SET gold=?,xp=?,level=?,updated_at=NOW() WHERE id=?")
        .bind(gold)
        .bind(xp)
        .bind(level)
        .bind(id)
        .execute(&mut **tx)
        .await?;
    crate::pets::award_xp(tx, id, xp_reward).await?;
    let tokens: i32 = sqlx::query_scalar("SELECT tokens FROM users WHERE id=?")
        .bind(row.try_get::<u64, _>("user_id")?)
        .fetch_one(&mut **tx)
        .await?;
    Ok((xp_reward, level, xp, level > old_level, tokens))
}
