use crate::{
    account::{guard, integer, string},
    inventory,
};
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};
type Result = std::result::Result<Json, sqlx::Error>;
pub async fn config(pool: &MySqlPool, key: &str) -> std::result::Result<Json, sqlx::Error> {
    Ok(
        sqlx::query_scalar::<_, Json>("SELECT value FROM game_configs WHERE `key`=?")
            .bind(key)
            .fetch_optional(pool)
            .await?
            .unwrap_or(Json::Null),
    )
}
fn denied(message: &str) -> Json {
    json!({"status":2,"result":message})
}
fn talent_type<'a>(description: &'a Json, skill: &str) -> Option<&'a str> {
    description.as_object()?.iter().find_map(|(key, v)| {
        if v["talent_skill_id"] == skill {
            key.strip_prefix("talent_")?
                .rsplit_once("_skill_")
                .map(|(kind, _)| kind)
        } else {
            None
        }
    })
}
pub async fn dispatch(pool: &MySqlPool, method: &str, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(e) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(e);
    }
    let description = config(pool, "talent_description").await?;
    if method == "getTalentSkills" {
        let rows = sqlx::query(
            "SELECT skill_id,level FROM character_talents WHERE character_id=? ORDER BY id",
        )
        .bind(id)
        .fetch_all(pool)
        .await?;
        let data:Vec<_>=rows.iter().map(|r|{let skill=r.get::<String,_>("skill_id");json!({"item_id":skill,"item_level":r.get::<i32,_>("level"),"talent_type":talent_type(&description,&skill)})}).collect();
        return Ok(json!({"status":1,"data":data}));
    }
    let info = config(pool, "talent_info").await?;
    let shop = config(pool, "talent_shop").await?;
    let costs = config(pool, "talent_tp_cost").await?;
    let mut tx = pool.begin().await?;
    let char=sqlx::query("SELECT c.*,u.tokens,u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE").bind(id).fetch_one(&mut *tx).await?;
    let selected: [Option<String>; 3] = [
        char.try_get("talent_1")?,
        char.try_get("talent_2")?,
        char.try_get("talent_3")?,
    ]
    .map(|slot: Option<String>| slot.filter(|v| !v.is_empty() && v != "0"));
    let result = match method {
        "discoverTalent" => {
            let mode = string(args, 2);
            let talent = string(args, 3);
            let mode_key = match mode.as_str() {
                "Extreme" => "extreme",
                "Secret" => "secret",
                _ => return Ok(denied("Invalid mode")),
            };
            if !shop[mode_key]
                .as_array()
                .is_some_and(|list| list.iter().any(|v| v["id"] == talent))
            {
                return Ok(denied("Talent not found"));
            }
            if selected.iter().any(|v| v.as_deref() == Some(&talent)) {
                return Ok(denied("Talent already discovered"));
            }
            let slot = if mode_key == "extreme" {
                if selected[0].is_some() {
                    return Ok(denied("Extreme slot full!"));
                }
                1
            } else if selected[1].is_none() {
                2
            } else if selected[2].is_none() {
                if char.try_get::<i32, _>("rank")? < 6 {
                    return Ok(denied(
                        "Reach Special Jounin to discover a second Secret talent.",
                    ));
                }
                3
            } else {
                return Ok(denied("All Secret slots full!"));
            };
            let Some(talent_info) = info.get(&talent) else {
                return Ok(denied("Talent data not found"));
            };
            if talent_info["is_emblem"] == true && char.try_get::<i8, _>("account_type")? == 0 {
                return Ok(json!({"status":6,"result":"Premium required!"}));
            }
            let gold = talent_info["price_gold"].as_i64().unwrap_or(0);
            let tokens = talent_info["price_token"].as_i64().unwrap_or(0);
            if gold < 0
                || tokens < 0
                || gold > i64::from(char.try_get::<i32, _>("gold")?)
                || tokens > i64::from(char.try_get::<i32, _>("tokens")?)
            {
                return Ok(denied("Not enough resources!"));
            }
            sqlx::query(&format!(
                "UPDATE characters SET talent_{slot}=?,gold=gold-?,updated_at=NOW() WHERE id=?"
            ))
            .bind(talent)
            .bind(gold)
            .bind(id)
            .execute(&mut *tx)
            .await?;
            sqlx::query("UPDATE users SET tokens=tokens-?,updated_at=NOW() WHERE id=?")
                .bind(tokens)
                .bind(char.try_get::<u64, _>("user_id")?)
                .execute(&mut *tx)
                .await?;
            json!({"status":1,"newt":slot,"tokens":i64::from(char.try_get::<i32,_>("tokens")?)-tokens,"golds":i64::from(char.try_get::<i32,_>("gold")?)-gold})
        }
        "buyPackageTP" => {
            let (amount, cost) = match integer(args, 2) {
                0 => (20, 20),
                1 => (125, 100),
                2 => (250, 200),
                3 => (600, 400),
                _ => return Ok(denied("Invalid package")),
            };
            if char.try_get::<i32, _>("tokens")? < cost
                || char.try_get::<i32, _>("tp")? > i32::MAX - amount
            {
                return Ok(denied("Not enough resources!"));
            }
            sqlx::query("UPDATE characters SET tp=tp+?,updated_at=NOW() WHERE id=?")
                .bind(amount)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE users SET tokens=tokens-?,updated_at=NOW() WHERE id=?")
                .bind(cost)
                .bind(char.try_get::<u64, _>("user_id")?)
                .execute(&mut *tx)
                .await?;
            json!({"status":1,"price":cost,"add":amount})
        }
        "upgradeSkill" => {
            let skill = string(args, 2);
            let Some(kind) = talent_type(&description, &skill) else {
                return Ok(denied("Talent skill not found"));
            };
            if !selected.iter().any(|v| v.as_deref() == Some(kind)) {
                return Ok(denied("Discover this talent first!"));
            }
            let row=sqlx::query("SELECT id,level FROM character_talents WHERE character_id=? AND skill_id=? LIMIT 1 FOR UPDATE").bind(id).bind(&skill).fetch_optional(&mut *tx).await?;
            let current = row.as_ref().map(|r| r.get::<i32, _>("level")).unwrap_or(0);
            if current >= 10 {
                return Ok(denied("Already at maximum level!"));
            }
            let max = integer(args, 3) == 1 || args.get(3) == Some(&Json::Bool(true));
            let tp = char.try_get::<i32, _>("tp")?;
            let mut spent = 0;
            let mut level = current;
            for next in current + 1..=10 {
                let cost = costs
                    .get(next.to_string())
                    .and_then(Json::as_i64)
                    .unwrap_or([5, 10, 25, 50, 100, 200, 300, 450, 600, 800][(next - 1) as usize]);
                if cost < 0 || i64::from(spent) + cost > i64::from(tp) {
                    break;
                }
                spent += cost as i32;
                level = next;
                if !max {
                    break;
                }
            }
            if level == current {
                return Ok(denied("Not enough TP!"));
            }
            if let Some(row) = row {
                sqlx::query("UPDATE character_talents SET level=?,updated_at=NOW() WHERE id=?")
                    .bind(level)
                    .bind(row.try_get::<u64, _>("id")?)
                    .execute(&mut *tx)
                    .await?;
            } else {
                sqlx::query("INSERT INTO character_talents (character_id,skill_id,level,created_at,updated_at) VALUES (?,?,?,NOW(),NOW())").bind(id).bind(skill).bind(level).execute(&mut *tx).await?;
            }
            sqlx::query("UPDATE characters SET tp=tp-?,updated_at=NOW() WHERE id=?")
                .bind(spent)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            json!({"status":1,"current_tp":tp-spent,"result":"Talent upgraded successfully!"})
        }
        "resetTalents" => {
            let Some(talents) = args
                .get(2)
                .and_then(Json::as_array)
                .filter(|v| !v.is_empty())
            else {
                return Ok(denied("No talents selected"));
            };
            if talents
                .iter()
                .any(|v| !selected.iter().any(|s| s.as_deref() == v.as_str()))
            {
                return Ok(denied("Talent not owned"));
            }
            if !inventory::take(&mut tx, id, "essential_02", 5).await? {
                return Ok(denied("Not enough Ninja Seal Gan!"));
            }
            let mut slots = selected;
            for talent in talents {
                for slot in &mut slots {
                    if slot.as_deref() == talent.as_str() {
                        *slot = None;
                    }
                }
                for (key, v) in description.as_object().into_iter().flatten() {
                    if key.starts_with(&format!("talent_{}_skill_", talent.as_str().unwrap_or("")))
                        && let Some(skill) = v["talent_skill_id"].as_str()
                    {
                        sqlx::query(
                            "DELETE FROM character_talents WHERE character_id=? AND skill_id=?",
                        )
                        .bind(id)
                        .bind(skill)
                        .execute(&mut *tx)
                        .await?;
                    }
                }
            }
            if slots[1].is_none() {
                slots[1] = slots[2].take();
            }
            sqlx::query("UPDATE characters SET talent_1=?,talent_2=?,talent_3=?,updated_at=NOW() WHERE id=?").bind(&slots[0]).bind(&slots[1]).bind(&slots[2]).bind(id).execute(&mut *tx).await?;
            json!({"status":1})
        }
        _ => return Ok(denied("Unknown talent operation")),
    };
    let rows = sqlx::query(
        "SELECT skill_id,level FROM character_talents WHERE character_id=? ORDER BY id",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;
    let skills = rows
        .iter()
        .map(|r| {
            format!(
                "{}:{}",
                r.get::<String, _>("skill_id"),
                r.get::<i32, _>("level")
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    sqlx::query("UPDATE characters SET talent_skills=? WHERE id=?")
        .bind(skills)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(result)
}
