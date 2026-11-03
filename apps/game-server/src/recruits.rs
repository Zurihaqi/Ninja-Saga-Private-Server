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
    let recruits: Vec<_> = recruits
        .into_iter()
        .filter(|r| r.starts_with("npc_"))
        .map(|r| json!({"type":"npc","id":{"recruiter_id":r}}))
        .collect();
    Ok(json!({"status":1,"error":0,"recruit":recruits,"daily":[]}))
}
