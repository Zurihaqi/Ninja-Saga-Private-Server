use crate::account::{guard, integer, string};
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};
type Result = std::result::Result<Json, sqlx::Error>;

pub async fn recruit(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(e) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(e);
    }
    let mut tx = pool.begin().await?;
    let char=sqlx::query("SELECT c.gold,c.user_id,u.tokens,u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE").bind(id).fetch_one(&mut *tx).await?;
    let recruits: Vec<String> = sqlx::query_scalar(
        "SELECT recruit_id FROM character_recruits WHERE character_id=? ORDER BY id",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;
    let requested = string(args, 2);
    if recruits.len() >= 2 || recruits.contains(&requested) {
        return Ok(json!({"status":2,"result":"You cannot recruit more teammate"}));
    }
    let Some(npc) = sqlx::query("SELECT * FROM npcs WHERE npc_id=?")
        .bind(&requested)
        .fetch_optional(&mut *tx)
        .await?
    else {
        return Ok(json!({"status":2,"result":"NPC not found"}));
    };
    if npc.try_get::<bool, _>("premium")? && char.try_get::<i8, _>("account_type")? == 0 {
        return Ok(json!({"status":2,"result":"You need to be premium user to recruit this NPC!"}));
    }
    let gold = npc.try_get::<i32, _>("price_gold")?;
    let tokens = npc.try_get::<i32, _>("price_tokens")?;
    if gold < 0
        || tokens < 0
        || gold > char.try_get::<i32, _>("gold")?
        || tokens > char.try_get::<i32, _>("tokens")?
    {
        return Ok(json!({"status":2,"result":"Not enough resources!"}));
    }
    sqlx::query("UPDATE characters SET gold=gold-?,updated_at=NOW() WHERE id=?")
        .bind(gold)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE users SET tokens=tokens-?,updated_at=NOW() WHERE id=?")
        .bind(tokens)
        .bind(char.try_get::<u64, _>("user_id")?)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO character_recruits (character_id,recruit_id,created_at,updated_at) VALUES (?,?,NOW(),NOW())").bind(id).bind(&requested).execute(&mut *tx).await?;
    let mut recruits = recruits;
    recruits.push(requested);
    let hash = authentication::registration_hash(&recruits[0]);
    tx.commit().await?;
    Ok(json!({"status":1,"recruiters":[recruits,hash]}))
}
pub async fn remove(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(e) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(e);
    }
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT id FROM characters WHERE id=? FOR UPDATE")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM character_recruits WHERE character_id=?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(json!({"status":1,"result":"Recruited squad removed."}))
}
pub async fn room(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(e) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(e);
    }
    let recruits: Vec<String> = sqlx::query_scalar(
        "SELECT recruit_id FROM character_recruits WHERE character_id=? ORDER BY id",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    let mut recruit_data = Vec::new();
    for r in recruits {
        let mut is_npc = r.starts_with("npc_");
        let mut normalized = r.clone();
        if !is_npc {
            let exact: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM npcs WHERE npc_id=?")
                .bind(&r)
                .fetch_one(pool)
                .await?;
            if exact > 0 {
                is_npc = true;
            } else {
                let prefixed = format!("npc_{r}");
                let pref_count: i64 =
                    sqlx::query_scalar("SELECT COUNT(*) FROM npcs WHERE npc_id=?")
                        .bind(&prefixed)
                        .fetch_one(pool)
                        .await?;
                if pref_count > 0 {
                    is_npc = true;
                    normalized = prefixed;
                }
            }
        }
        if is_npc {
            recruit_data.push(json!({"type":"npc","id":{"recruiter_id":normalized}}));
        } else {
            let friend_id_str = r.strip_prefix("char_").unwrap_or(&r);
            if let Ok(friend_id) = friend_id_str.parse::<i64>()
                && let Some(friend) = sqlx::query(
                    "SELECT c.*, u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=?",
                )
                .bind(friend_id)
                .fetch_optional(pool)
                .await?
            {
                let gender: i8 = friend.try_get("gender").unwrap_or(0);
                let suffix = if gender == 0 { "_0" } else { "_1" };
                let hair_raw: Option<String> = friend.try_get("hair_style").ok().flatten();
                let hair = match hair_raw.as_deref() {
                    Some(h) if !h.is_empty() && h.chars().all(|c| c.is_ascii_digit()) => {
                        format!("hair_{:0>2}{suffix}", h)
                    }
                    Some(h) if !h.is_empty() => h.to_string(),
                    _ => format!("hair_01{suffix}"),
                };
                let weapon = friend
                    .try_get::<Option<String>, _>("equipment_weapon")?
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "wpn_01".into());
                let back = friend
                    .try_get::<Option<String>, _>("equipment_back")?
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "back_01".into());
                let clothing = friend
                    .try_get::<Option<String>, _>("equipment_clothing")?
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| format!("set_01{suffix}"));
                let accessory = friend
                    .try_get::<Option<String>, _>("equipment_accessory")?
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "accessory_01".into());
                let hair_color = friend
                    .try_get::<Option<String>, _>("hair_color")?
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "0|0".into());
                let skin_color = friend
                    .try_get::<Option<String>, _>("skin_color")?
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "0|0".into());
                let pet_val = match friend
                    .try_get::<Option<String>, _>("equipment_pet")?
                    .filter(|s| !s.is_empty())
                {
                    Some(p) => json!(p),
                    None => json!(0),
                };
                recruit_data.push(json!({
                    "type": "char",
                    "id": friend.try_get::<u64, _>("id")?,
                    "info": {
                        "name": friend.try_get::<String, _>("name")?,
                        "level": friend.try_get::<i32, _>("level")?,
                        "rank": friend.try_get::<i8, _>("rank")?,
                        "emblem": friend.try_get::<i8, _>("account_type").unwrap_or(0) == 1,
                        "element_1": friend.try_get::<i8, _>("element_1").unwrap_or(0),
                        "element_2": friend.try_get::<i8, _>("element_2").unwrap_or(0),
                        "element_3": friend.try_get::<i8, _>("element_3").unwrap_or(0),
                        "set": {
                            "weapon": weapon,
                            "back_item": back,
                            "clothing": clothing,
                            "hairstyle": hair,
                            "accessory": accessory,
                            "face": format!("face_01{suffix}"),
                            "hair_color": hair_color,
                            "skin_color": skin_color,
                            "pet": pet_val
                        }
                    }
                }));
            }
        }
    }
    Ok(json!({"status":1,"error":0,"recruit":recruit_data,"daily":[]}))
}
