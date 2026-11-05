use crate::account::{guard, integer, string};
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};
type Result = std::result::Result<Json, sqlx::Error>;
fn denied(message: &str) -> Json {
    json!({"status":2,"result":message})
}
pub fn valid_class(skill: &str) -> bool {
    matches!(
        skill,
        "skill_4000" | "skill_4001" | "skill_4002" | "skill_4003" | "skill_4004"
    )
}
pub async fn change_class(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(e) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(e);
    }
    let skill = string(args, 2);
    if !valid_class(&skill) {
        return Ok(denied("Invalid class selection."));
    }
    let mut tx = pool.begin().await?;
    let char=sqlx::query("SELECT c.*,u.tokens,u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE").bind(id).fetch_one(&mut *tx).await?;
    if char.try_get::<i32, _>("level")? < 60 || char.try_get::<i32, _>("rank")? < 7 {
        return Ok(denied("You must be a Special Jounin to change your class!"));
    }
    let old = char.try_get::<Option<String>, _>("class")?;
    if old.as_deref() == Some(&skill) {
        return Ok(denied("You already have this class selected."));
    }
    let price = if old.is_none() {
        0
    } else if char.try_get::<i8, _>("account_type")? > 0 {
        2000
    } else {
        3000
    };
    if char.try_get::<i32, _>("tokens")? < price {
        return Ok(denied("Not enough tokens!"));
    }
    sqlx::query("UPDATE characters SET `class`=?,updated_at=NOW() WHERE id=?")
        .bind(&skill)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE users SET tokens=tokens-?,updated_at=NOW() WHERE id=?")
        .bind(price)
        .bind(char.try_get::<u64, _>("user_id")?)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(json!({"status":1,"result":"Class changed successfully!"}))
}
pub async fn upgrade(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(e) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(e);
    }
    let skill = string(args, 2);
    let chains = crate::talents::config(pool, "academy_chains").await?;
    let previous = chains
        .as_object()
        .into_iter()
        .flatten()
        .flat_map(|(_, element)| element.as_object().into_iter().flatten())
        .filter_map(|(_, chain)| chain.as_array())
        .find_map(|chain| {
            chain.iter().position(|v| v == &skill).map(|index| {
                if index > 0 {
                    chain[index - 1].as_str().map(str::to_owned)
                } else {
                    None
                }
            })
        });
    let Some(previous) = previous else {
        return Ok(denied("This skill cannot be upgraded here!"));
    };
    let mut tx = pool.begin().await?;
    let char=sqlx::query("SELECT c.*,u.tokens,u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE").bind(id).fetch_one(&mut *tx).await?;
    let Some(config) = sqlx::query("SELECT * FROM skills WHERE skill_id=?")
        .bind(&skill)
        .fetch_optional(&mut *tx)
        .await?
    else {
        return Ok(denied("Skill info not found in database!"));
    };
    let owned: Vec<String> =
        sqlx::query_scalar("SELECT skill_id FROM character_skills WHERE character_id=?")
            .bind(id)
            .fetch_all(&mut *tx)
            .await?;
    if owned.contains(&skill) {
        return Ok(denied("Skill already learned!"));
    }
    if previous.as_ref().is_some_and(|p| !owned.contains(p)) {
        return Ok(denied("You do not own the prerequisite skill!"));
    }
    if config.try_get::<i32, _>("level")? > char.try_get::<i32, _>("level")? {
        return Ok(denied("Level too low!"));
    }
    if config.try_get::<bool, _>("premium")? && char.try_get::<i8, _>("account_type")? == 0 {
        return Ok(json!({"status":6}));
    }
    let gold = config.try_get::<i32, _>("price_gold")?;
    let tokens = config.try_get::<i32, _>("price_tokens")?;
    if gold < 0
        || tokens < 0
        || char.try_get::<i32, _>("gold")? < gold
        || char.try_get::<i32, _>("tokens")? < tokens
    {
        return Ok(denied("Not enough resources!"));
    }
    let mut equipped = char
        .try_get::<Option<String>, _>("equipment_skills")?
        .unwrap_or_default();
    if let Some(previous) = previous {
        sqlx::query("DELETE FROM character_skills WHERE character_id=? AND skill_id=?")
            .bind(id)
            .bind(&previous)
            .execute(&mut *tx)
            .await?;
        equipped = equipped
            .split(',')
            .map(|s| if s == previous { skill.as_str() } else { s })
            .collect::<Vec<_>>()
            .join(",");
        let presets =
            sqlx::query("SELECT id,skills FROM character_skill_sets WHERE character_id=?")
                .bind(id)
                .fetch_all(&mut *tx)
                .await?;
        for preset in presets {
            let skills = preset
                .try_get::<Option<String>, _>("skills")?
                .unwrap_or_default()
                .split(',')
                .map(|s| if s == previous { skill.as_str() } else { s })
                .collect::<Vec<_>>()
                .join(",");
            sqlx::query("UPDATE character_skill_sets SET skills=? WHERE id=?")
                .bind(skills)
                .bind(preset.try_get::<u64, _>("id")?)
                .execute(&mut *tx)
                .await?;
        }
    }
    sqlx::query("INSERT INTO character_skills (character_id,skill_id,created_at,updated_at) VALUES (?,?,NOW(),NOW())").bind(id).bind(skill).execute(&mut *tx).await?;
    sqlx::query("UPDATE characters SET gold=gold-?,equipment_skills=?,updated_at=NOW() WHERE id=?")
        .bind(gold)
        .bind(&equipped)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE users SET tokens=tokens-?,updated_at=NOW() WHERE id=?")
        .bind(tokens)
        .bind(char.try_get::<u64, _>("user_id")?)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(
        json!({"status":1,"result":"Skill upgraded successfully!","account_tokens":char.try_get::<i32,_>("tokens")?-tokens,"character_skills":equipped,"character_set_skills":equipped}),
    )
}
