//! Inventory pet actions, using the original client learning costs and food MP.
use crate::{
    account::{integer, string},
    inventory,
};
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};
type Result = std::result::Result<Json, sqlx::Error>;
fn denied(message: &str) -> Json {
    json!({"status":2,"result":message})
}

pub async fn dispatch(pool: &MySqlPool, method: &str, args: &[Json]) -> Result {
    // Caller has already validated the character session.
    let id = integer(args, 0);
    let pet_id = integer(args, 2);
    let mut tx = pool.begin().await?;
    let char=sqlx::query("SELECT c.level,c.gold,c.user_id,u.tokens,u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE").bind(id).fetch_one(&mut *tx).await?;
    let Some(pet)=sqlx::query("SELECT p.*,c.skills AS catalog_skills FROM character_pets p LEFT JOIN pets c ON c.pet_id=p.pet_id WHERE p.id=? AND p.character_id=? FOR UPDATE").bind(pet_id).bind(id).fetch_optional(&mut *tx).await? else {return Ok(denied("Pet not found in your inventory"));};
    let in_training:i64=sqlx::query_scalar("SELECT COUNT(*) FROM pet_villa_slots WHERE character_id=? AND pet_instance_id=? AND status IN (2,3)").bind(id).bind(pet_id).fetch_one(&mut *tx).await?;
    if in_training > 0 {
        return Ok(denied("Check this pet out of training first."));
    }
    let result = match method {
        "feed" => {
            let food = string(args, 3);
            let Some(config) = crate::economy::catalog_entry("library", &food) else {
                return Ok(denied("Food not found"));
            };
            let mp = config["mp"].as_i64().unwrap_or(0);
            if mp <= 0 || mp > 100 {
                return Ok(denied("This item is not pet food"));
            }
            let current = pet.try_get::<u32, _>("maturity_points")?;
            if current >= 100 {
                return Ok(denied("Pet maturity is already full"));
            }
            if !inventory::take(&mut tx, id, &food, 1).await? {
                return Ok(denied("Food not found in inventory"));
            }
            let maturity = (i64::from(current) + mp).min(100);
            sqlx::query("UPDATE character_pets SET maturity_points=?,updated_at=NOW() WHERE id=?")
                .bind(maturity)
                .bind(pet_id)
                .execute(&mut *tx)
                .await?;
            json!({"status":1,"mp":maturity})
        }
        "learnSkill" => {
            let slot = integer(args, 3);
            let (gold, tokens, mats) = match slot {
                2 => (10000, 50, [0, 0, 0, 0, 0]),
                3 => (20000, 80, [5, 0, 0, 0, 0]),
                4 => (30000, 120, [6, 4, 2, 0, 0]),
                5 => (40000, 150, [8, 6, 4, 0, 0]),
                6 => (50000, 200, [10, 8, 6, 4, 2]),
                _ => return Ok(denied("Invalid pet skill slot")),
            };
            let skills = pet.try_get::<Option<Json>, _>("catalog_skills")?;
            let required = skills
                .as_ref()
                .and_then(Json::as_array)
                .and_then(|a| a.get((slot - 1) as usize))
                .and_then(|s| s["level"].as_i64())
                .unwrap_or((slot - 1) * 10);
            if i64::from(pet.try_get::<i32, _>("level")?) < required {
                return Ok(denied("Pet level too low"));
            }
            let learned: Vec<u8> = sqlx::query_scalar(
                "SELECT skill_slot FROM character_pet_skills WHERE pet_instance_id=?",
            )
            .bind(pet_id)
            .fetch_all(&mut *tx)
            .await?;
            if learned.is_empty() || learned.contains(&(slot as u8)) {
                return Ok(denied("Pet skill already learned"));
            }
            match string(args, 4).as_str() {
                "mc1" => {
                    if char.try_get::<i32, _>("gold")? < gold {
                        return Ok(denied("Not enough gold"));
                    }
                    for (i, qty) in mats.iter().enumerate().filter(|(_, q)| **q > 0) {
                        if !inventory::take(&mut tx, id, &format!("material_{:02}", i + 1), *qty)
                            .await?
                        {
                            return Ok(denied("Not enough materials"));
                        }
                    }
                    sqlx::query("UPDATE characters SET gold=gold-?,updated_at=NOW() WHERE id=?")
                        .bind(gold)
                        .bind(id)
                        .execute(&mut *tx)
                        .await?;
                }
                "mc2" => {
                    if char.try_get::<i32, _>("tokens")? < tokens {
                        return Ok(denied("Not enough tokens"));
                    }
                    sqlx::query("UPDATE users SET tokens=tokens-?,updated_at=NOW() WHERE id=?")
                        .bind(tokens)
                        .bind(char.try_get::<u64, _>("user_id")?)
                        .execute(&mut *tx)
                        .await?;
                }
                _ => return Ok(denied("Invalid learning method")),
            }
            sqlx::query(
                "INSERT INTO character_pet_skills (pet_instance_id,skill_slot) VALUES (?,?)",
            )
            .bind(pet_id)
            .bind(slot)
            .execute(&mut *tx)
            .await?;
            json!({"status":1,"result":"Pet skill learned"})
        }
        "setLevel" => {
            // This unbound client developer action has no retail price/contract.
            // Match the permission required by Character.setCharInfo, rather than
            // granting arbitrary free pet levels to ordinary players.
            if char.try_get::<i8, _>("account_type")? != 99 {
                return Ok(denied("Administrator access required"));
            }
            let level = char.try_get::<i32, _>("level")?.clamp(1, 85);
            if pet.try_get::<i32, _>("level")? >= level {
                return Ok(denied("Pet is already at your level"));
            }
            let xp: i64 = if level == 1 {
                0
            } else {
                sqlx::query_scalar("SELECT pet_xp FROM x_p_s WHERE level=?")
                    .bind(level - 1)
                    .fetch_optional(&mut *tx)
                    .await?
                    .unwrap_or(0)
            };
            sqlx::query("UPDATE character_pets SET level=?,xp=?,updated_at=NOW() WHERE id=?")
                .bind(level)
                .bind(xp)
                .bind(pet_id)
                .execute(&mut *tx)
                .await?;
            json!({"status":1,"level":level})
        }
        _ => return Ok(denied("Unknown pet action")),
    };
    tx.commit().await?;
    Ok(result)
}
