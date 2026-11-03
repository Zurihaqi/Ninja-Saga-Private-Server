//! Skill preset operations used by the normal skills panel.
use crate::account::{guard, integer, string};
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};
type Result = std::result::Result<Json, sqlx::Error>;
pub async fn list(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(e) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(e);
    }
    let rows = sqlx::query(
        "SELECT id,skills FROM character_skill_sets WHERE character_id=? ORDER BY preset_index,id",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    let sets:Vec<_>=rows.iter().map(|r|json!({"id":r.get::<u64,_>("id"),"skills":r.get::<Option<String>,_>("skills").unwrap_or_default()})).collect();
    Ok(json!({"status":1,"skillsets":sets}))
}
pub async fn create(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(e) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(e);
    }
    let mut tx = pool.begin().await?;
    let row=sqlx::query("SELECT c.user_id,c.equipment_skills,u.account_type,u.tokens FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE").bind(id).fetch_one(&mut *tx).await?;
    if row.try_get::<i8, _>("account_type")? == 0 {
        return Ok(json!({"status":2,"result":"Premium required to unlock Skill Sets!"}));
    }
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM character_skill_sets WHERE character_id=?")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    if count >= 4 {
        return Ok(json!({"status":2,"result":"Maximum 4 presets allowed!"}));
    }
    let cost = count * 100;
    let tokens = i64::from(row.try_get::<i32, _>("tokens")?);
    if tokens < cost {
        return Ok(json!({"status":2,"result":"Not enough tokens!"}));
    }
    sqlx::query("UPDATE users SET tokens=?,updated_at=NOW() WHERE id=?")
        .bind(tokens - cost)
        .bind(row.try_get::<u64, _>("user_id")?)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO character_skill_sets(character_id,preset_index,skills,created_at,updated_at) VALUES(?,?,?,NOW(),NOW())").bind(id).bind(count+1).bind(row.try_get::<Option<String>,_>("equipment_skills")?).execute(&mut *tx).await?;
    tx.commit().await?;
    list(pool, args).await
}
pub async fn save(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(e) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(e);
    }
    let preset = integer(args, 2);
    let skills = string(args, 3);
    if skills.len() > 4096 || skills.split(',').count() > 8 {
        return Ok(json!({"status":2,"error":"Invalid skill set"}));
    }
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT id FROM characters WHERE id=? FOR UPDATE")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    let exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM character_skill_sets WHERE character_id=? AND id=?",
    )
    .bind(id)
    .bind(preset)
    .fetch_one(&mut *tx)
    .await?;
    if exists == 0 {
        return Ok(json!({"status":2,"result":"Preset not found!"}));
    }
    for skill in skills.split(',').filter(|s| !s.is_empty()) {
        let owned: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM character_skills WHERE character_id=? AND skill_id=?",
        )
        .bind(id)
        .bind(skill)
        .fetch_one(&mut *tx)
        .await?;
        if owned == 0 {
            return Ok(json!({"status":2,"error":"Skill is not owned"}));
        }
    }
    sqlx::query(
        "UPDATE character_skill_sets SET skills=?,updated_at=NOW() WHERE character_id=? AND id=?",
    )
    .bind(&skills)
    .bind(id)
    .bind(preset)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE characters SET equipment_skills=?,updated_at=NOW() WHERE id=?")
        .bind(skills)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    list(pool, args).await
}
