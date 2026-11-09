use crate::account::{guard, integer, string};
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};
type Result = std::result::Result<Json, sqlx::Error>;

pub async fn list_data(pool: &MySqlPool, id: i64) -> std::result::Result<Vec<Json>, sqlx::Error> {
    let rows=sqlx::query("SELECT p.*,c.skills,c.swf,c.name AS catalog_name,l.learned_slots FROM character_pets p LEFT JOIN pets c ON c.pet_id=p.pet_id JOIN characters ch ON ch.id=p.character_id LEFT JOIN (SELECT pet_instance_id,GROUP_CONCAT(skill_slot ORDER BY skill_slot) AS learned_slots FROM character_pet_skills GROUP BY pet_instance_id) l ON l.pet_instance_id=p.id WHERE p.character_id=? ORDER BY (p.id=ch.equipment_pet) DESC,p.id").bind(id).fetch_all(pool).await?;
    rows.iter().map(|r| {
        let level:i32=r.try_get("level")?;
        let skills=r.try_get::<Option<Json>,_>("skills")?.and_then(|v|v.as_array().cloned());
        let mut flags:Vec<_>=if let Some(skills)=skills.filter(|v|!v.is_empty()) {skills.iter().map(|s|if i64::from(level)>=s["level"].as_i64().unwrap_or(1) {"1"}else{"0"}).collect()}else{[1,10,20,30,40,50].iter().map(|required|if level>=*required {"1"}else{"0"}).collect()};
        flags.resize(flags.len().max(6),"0");
        // Pets predating explicit learning retain their legacy level-based unlocks.
        // Newly purchased pets have a basic-skill marker and learn additional slots.
        if let Some(learned)=r.try_get::<Option<String>,_>("learned_slots")? {
            flags=(1..=6).map(|slot|if learned.split(',').any(|s|s==slot.to_string()) {"1"}else{"0"}).collect();
        }
        Ok(json!({"pet":r.try_get::<u64,_>("id")?,"char_id":id,"pet_id":r.try_get::<u64,_>("id")?,"pet_name":r.try_get::<Option<String>,_>("name")?.or(r.try_get::<Option<String>,_>("catalog_name")?).unwrap_or_else(||"Pet".into()),"pet_xp":r.try_get::<i64,_>("xp")?,"pet_level":level,"pet_skills":flags.join(","),"pet_mp":r.try_get::<u32,_>("maturity_points")?,"pet_swf":r.try_get::<Option<String>,_>("swf")?.unwrap_or(r.try_get::<String,_>("pet_id")?)}))
    }).collect()
}
fn error(message: &str) -> Json {
    json!({"status":0,"error":message})
}
pub async fn dispatch(pool: &MySqlPool, method: &str, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(e) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(e);
    }
    if method == "getPets" {
        return Ok(json!({"status":1,"pets":list_data(pool,id).await?}));
    }
    if matches!(method, "feed" | "learnSkill" | "setLevel") {
        return crate::pet_actions::dispatch(pool, method, args).await;
    }
    if matches!(
        method,
        "getVillaData"
            | "trainPet"
            | "skipTraining"
            | "cancelTraining"
            | "checkoutPet"
            | "unlockSlots"
    ) {
        return dispatch_villa(pool, method, args).await;
    }
    let mut tx = pool.begin().await?;
    let char=sqlx::query("SELECT c.*,u.tokens,u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE").bind(id).fetch_one(&mut *tx).await?;
    if method == "buyPet" {
        let Some(config) = sqlx::query("SELECT * FROM pets WHERE pet_id=?")
            .bind(string(args, 2))
            .fetch_optional(&mut *tx)
            .await?
        else {
            return Ok(error("Pet data not found!"));
        };
        if config.try_get::<bool, _>("premium")? && char.try_get::<i8, _>("account_type")? == 0 {
            return Ok(json!({"status":6}));
        }
        let gold = char.try_get::<i32, _>("gold")?;
        let tokens = char.try_get::<i32, _>("tokens")?;
        let price = config.try_get::<i32, _>("price_gold")?;
        let token_price = config.try_get::<i32, _>("price_tokens")?;
        if price < 0 || token_price < 0 || gold < price || tokens < token_price {
            return Ok(json!({"status":3,"result":"Not enough resources!"}));
        }
        sqlx::query("UPDATE characters SET gold=gold-?,updated_at=NOW() WHERE id=?")
            .bind(price)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE users SET tokens=tokens-?,updated_at=NOW() WHERE id=?")
            .bind(token_price)
            .bind(char.try_get::<u64, _>("user_id")?)
            .execute(&mut *tx)
            .await?;
        let pet=sqlx::query("INSERT INTO character_pets (character_id,pet_id,name,level,xp,created_at,updated_at) VALUES (?,?,?,1,0,NOW(),NOW())").bind(id).bind(string(args,2)).bind(config.try_get::<String,_>("name")?).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO character_pet_skills (pet_instance_id,skill_slot) VALUES (?,1)")
            .bind(pet.last_insert_id())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Ok(
            json!({"status":1,"error":0,"data":{"character_gold":gold-price,"account_tokens":tokens-token_price}}),
        );
    }
    if method == "unequipPet" {
        sqlx::query("UPDATE characters SET equipment_pet=NULL,updated_at=NOW() WHERE id=?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Ok(json!({"status":1}));
    }
    let pet_id = integer(args, 2);
    let Some(pet)=sqlx::query("SELECT p.*,c.swf FROM character_pets p LEFT JOIN pets c ON c.pet_id=p.pet_id WHERE p.id=? AND p.character_id=? FOR UPDATE").bind(pet_id).bind(id).fetch_optional(&mut *tx).await? else{return Ok(error("Pet not found in your inventory"));};
    let response = match method {
        "equipPet" => {
            if char.try_get::<i32, _>("rank")? < 3 {
                return Ok(
                    json!({"status":2,"result":"You must be Chunin or higher to use pets!"}),
                );
            }
            sqlx::query("UPDATE characters SET equipment_pet=?,updated_at=NOW() WHERE id=?")
                .bind(pet_id)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            json!({"status":1,"pet_id":pet_id,"pet_swf":pet.try_get::<Option<String>,_>("swf")?.unwrap_or(pet.try_get("pet_id")?)})
        }
        "releasePet" => {
            if char
                .try_get::<Option<String>, _>("equipment_pet")?
                .and_then(|v| v.parse::<i64>().ok())
                == Some(pet_id)
            {
                return Ok(error("Cannot release an equipped pet! Unequip it first."));
            }
            sqlx::query("DELETE FROM character_pets WHERE id=?")
                .bind(pet_id)
                .execute(&mut *tx)
                .await?;
            json!({"status":1})
        }
        "renamePet" => {
            let name = string(args, 3).trim().to_owned();
            if name.is_empty() || name.chars().count() > 32 || name.chars().any(char::is_control) {
                return Ok(error("Invalid pet name"));
            }
            let required = if char.try_get::<i8, _>("account_type")? > 0 {
                1
            } else {
                3
            };
            let badge=sqlx::query("SELECT id,quantity FROM character_items WHERE character_id=? AND item_id='essential_01' LIMIT 1 FOR UPDATE").bind(id).fetch_optional(&mut *tx).await?;
            let Some(badge) =
                badge.filter(|r| r.try_get::<i32, _>("quantity").unwrap_or(0) >= required)
            else {
                return Ok(
                    json!({"status":2,"result":format!("You need {required} Rename Badge(s) to rename your pet.")}),
                );
            };
            sqlx::query("UPDATE character_pets SET name=?,updated_at=NOW() WHERE id=?")
                .bind(name)
                .bind(pet_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE character_items SET quantity=quantity-? WHERE id=?")
                .bind(required)
                .bind(badge.try_get::<u64, _>("id")?)
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM character_items WHERE character_id=? AND quantity=0")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            json!({"status":1})
        }
        _ => return Ok(error("Unknown pet operation")),
    };
    tx.commit().await?;
    Ok(response)
}

pub async fn award_xp(
    tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
    id: i64,
    reward: i64,
) -> std::result::Result<(), sqlx::Error> {
    let pet=sqlx::query("SELECT p.id,p.xp,p.level FROM character_pets p JOIN characters c ON c.equipment_pet=p.id AND c.id=p.character_id WHERE c.id=? FOR UPDATE").bind(id).fetch_optional(&mut **tx).await?;
    if let Some(pet) = pet {
        let xp = pet
            .try_get::<i64, _>("xp")?
            .checked_add(reward)
            .ok_or_else(|| sqlx::Error::Protocol("Pet XP overflow".into()))?;
        let mut level = pet.try_get::<i32, _>("level")?;
        while level < 85 {
            let required = sqlx::query_scalar::<_, i64>("SELECT pet_xp FROM x_p_s WHERE level=?")
                .bind(level)
                .fetch_optional(&mut **tx)
                .await?;
            if required.is_some_and(|r| r > 0 && xp >= r) {
                level += 1;
            } else {
                break;
            }
        }
        sqlx::query("UPDATE character_pets SET xp=?,level=?,updated_at=NOW() WHERE id=?")
            .bind(xp)
            .bind(level)
            .bind(pet.try_get::<u64, _>("id")?)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// Pet Villa (VillaService)
// -----------------------------------------------------------------------------

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn training_seconds(level: i32) -> i64 {
    i64::from(level.max(1)) * 1440
}

fn training_gold(level: i32) -> i32 {
    level.max(1).saturating_mul(2000)
}

fn skip_price(remaining_seconds: i64) -> i32 {
    if remaining_seconds <= 0 {
        1
    } else {
        i32::try_from((remaining_seconds + 59) / 60)
            .unwrap_or(1)
            .max(1)
    }
}

fn format_duration(seconds: i64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

async fn init_and_autocomplete_slots(
    pool: &MySqlPool,
    char_id: i64,
) -> std::result::Result<(), sqlx::Error> {
    for slot_index in 0..4i8 {
        let initial_status = if slot_index < 2 { 1i8 } else { 0i8 };
        sqlx::query(
            "INSERT IGNORE INTO pet_villa_slots (character_id, slot_index, status, pet_instance_id, training_ends_at, gold_spent, created_at, updated_at) VALUES (?, ?, ?, NULL, NULL, 0, NOW(), NOW())",
        )
        .bind(char_id)
        .bind(slot_index)
        .bind(initial_status)
        .execute(pool)
        .await?;
    }
    let now = now_unix();
    sqlx::query(
        "UPDATE pet_villa_slots SET status=3, updated_at=NOW() WHERE character_id=? AND status=2 AND training_ends_at<=?",
    )
    .bind(char_id)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

async fn dispatch_villa(pool: &MySqlPool, method: &str, args: &[Json]) -> Result {
    let char_id = integer(args, 0);
    init_and_autocomplete_slots(pool, char_id).await?;
    match method {
        "getVillaData" => {
            let now = now_unix();
            let slot_rows = sqlx::query(
                "SELECT slot_index, status, pet_instance_id, training_ends_at FROM pet_villa_slots WHERE character_id=? ORDER BY slot_index",
            )
            .bind(char_id)
            .fetch_all(pool)
            .await?;

            let mut slots_out = Vec::with_capacity(4);
            for i in 0..4i8 {
                let row = slot_rows
                    .iter()
                    .find(|r| r.try_get::<i8, _>("slot_index").ok() == Some(i));
                let status = row
                    .and_then(|r| r.try_get::<i8, _>("status").ok())
                    .unwrap_or(0);
                let pet_id = row
                    .and_then(|r| {
                        r.try_get::<Option<u64>, _>("pet_instance_id")
                            .ok()
                            .flatten()
                    })
                    .unwrap_or(0);
                let ends_at = row
                    .and_then(|r| {
                        r.try_get::<Option<u32>, _>("training_ends_at")
                            .ok()
                            .flatten()
                    })
                    .map(i64::from);

                let (remaining, skip_cost) = if status == 2
                    && let Some(ends) = ends_at
                {
                    let rem = (ends - now).max(0);
                    (rem, skip_price(rem))
                } else {
                    (0, 0)
                };

                slots_out.push(json!({
                    "status": status,
                    "pet_id": pet_id,
                    "skip_price": skip_cost,
                    "completed_at": remaining
                }));
            }

            let pet_rows = sqlx::query(
                "SELECT p.id, p.pet_id, p.name, p.level, p.xp, c.name AS catalog_name, c.swf AS catalog_swf FROM character_pets p LEFT JOIN pets c ON c.pet_id=p.pet_id WHERE p.character_id=? ORDER BY p.id",
            )
            .bind(char_id)
            .fetch_all(pool)
            .await?;

            let mut pets_out = Vec::with_capacity(pet_rows.len());
            for cp in pet_rows {
                let level: i32 = cp.try_get("level")?;
                let duration = training_seconds(level);
                let gold = training_gold(level);
                let name = cp
                    .try_get::<Option<String>, _>("name")?
                    .or(cp.try_get::<Option<String>, _>("catalog_name")?)
                    .unwrap_or_else(|| "Unknown Pet".into());
                pets_out.push(json!({
                    "pet_id": cp.try_get::<u64, _>("id")?,
                    "pet_name": name,
                    "pet_level": level,
                    "pet_xp": cp.try_get::<i64, _>("xp")?,
                    "pet_swf": cp.try_get::<Option<String>, _>("catalog_swf")?.filter(|v|!v.is_empty()).unwrap_or(cp.try_get::<String,_>("pet_id")?),
                    "train_time_text": format_duration(duration),
                    "train_gold": gold
                }));
            }

            Ok(json!({
                "status": 1,
                "slots": slots_out,
                "pets": pets_out
            }))
        }
        "trainPet" => {
            let pet_instance_id = integer(args, 2);
            let mut tx = pool.begin().await?;
            // All pet writes lock the owner before pets/slots, so training cannot
            // race with feeding, checkout or another start for this character.
            let owner = sqlx::query(
                "SELECT level,gold,equipment_pet FROM characters WHERE id=? FOR UPDATE",
            )
            .bind(char_id)
            .fetch_one(&mut *tx)
            .await?;
            let Some(pet) = sqlx::query(
                "SELECT id, level FROM character_pets WHERE id=? AND character_id=? FOR UPDATE",
            )
            .bind(pet_instance_id)
            .bind(char_id)
            .fetch_optional(&mut *tx)
            .await?
            else {
                return Ok(error("Pet not found."));
            };
            let in_slot: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM pet_villa_slots WHERE character_id=? AND pet_instance_id=? AND status IN (2, 3)",
            )
            .bind(char_id)
            .bind(pet_instance_id)
            .fetch_one(&mut *tx)
            .await?;
            if in_slot > 0 {
                return Ok(json!({
                    "status": 2,
                    "result": "This pet is already in training or waiting for checkout."
                }));
            }
            let Some(slot_id) = sqlx::query_scalar::<_, u64>(
                "SELECT id FROM pet_villa_slots WHERE character_id=? AND status=1 ORDER BY slot_index LIMIT 1 FOR UPDATE",
            )
            .bind(char_id)
            .fetch_optional(&mut *tx)
            .await?
            else {
                return Ok(json!({
                    "status": 2,
                    "result": "No available training slots. Unlock more slots or wait for current training to finish."
                }));
            };
            let level: i32 = pet.try_get("level")?;
            if level >= owner.try_get::<i32, _>("level")?.min(85) {
                return Ok(
                    json!({"status":2,"result":"This pet has reached your character's level cap."}),
                );
            }
            let gold_cost = training_gold(level);
            let char_gold: i32 = owner.try_get("gold")?;
            if char_gold < gold_cost {
                return Ok(json!({
                    "status": 2,
                    "result": format!("Not enough gold. Training requires {gold_cost} gold.")
                }));
            }
            let ends_at = now_unix() + training_seconds(level);
            sqlx::query("UPDATE characters SET gold=gold-?, updated_at=NOW() WHERE id=?")
                .bind(gold_cost)
                .bind(char_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "UPDATE pet_villa_slots SET status=2, pet_instance_id=?, training_ends_at=?, gold_spent=?, updated_at=NOW() WHERE id=?",
            )
            .bind(pet_instance_id)
            .bind(ends_at)
            .bind(gold_cost)
            .bind(slot_id)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            Ok(json!({"status": 1, "result": "Training started!", "gold":char_gold-gold_cost}))
        }
        "skipTraining" => {
            let pet_instance_id = integer(args, 2);
            let mut tx = pool.begin().await?;
            sqlx::query("SELECT id FROM characters WHERE id=? FOR UPDATE")
                .bind(char_id)
                .fetch_one(&mut *tx)
                .await?;
            let Some(slot) = sqlx::query(
                "SELECT id, training_ends_at FROM pet_villa_slots WHERE character_id=? AND pet_instance_id=? AND status=2 LIMIT 1 FOR UPDATE",
            )
            .bind(char_id)
            .bind(pet_instance_id)
            .fetch_optional(&mut *tx)
            .await?
            else {
                return Ok(error("Training slot not found."));
            };
            let now = now_unix();
            let ends_at = i64::from(
                slot.try_get::<Option<u32>, _>("training_ends_at")?
                    .unwrap_or(0),
            );
            let remaining = (ends_at - now).max(0);
            let cost = skip_price(remaining);
            let Some(char_row) = sqlx::query(
                "SELECT c.user_id, u.tokens FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE",
            )
            .bind(char_id)
            .fetch_optional(&mut *tx)
            .await?
            else {
                return Ok(error("Character not found."));
            };
            let tokens: i32 = char_row.try_get("tokens")?;
            if tokens < cost {
                return Ok(json!({
                    "status": 2,
                    "result": format!("Not enough tokens. Skip costs {cost} tokens.")
                }));
            }
            let user_id: u64 = char_row.try_get("user_id")?;
            sqlx::query("UPDATE users SET tokens=tokens-?, updated_at=NOW() WHERE id=?")
                .bind(cost)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "UPDATE pet_villa_slots SET status=3, training_ends_at=?, updated_at=NOW() WHERE id=?",
            )
            .bind(now)
            .bind(slot.try_get::<u64, _>("id")?)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            Ok(json!({
                "status": 1,
                "result": "Training skipped! Your pet is ready for checkout.",
                "tokens":tokens-cost
            }))
        }
        "cancelTraining" => {
            let pet_instance_id = integer(args, 2);
            let mut tx = pool.begin().await?;
            sqlx::query("SELECT id FROM characters WHERE id=? FOR UPDATE")
                .bind(char_id)
                .fetch_one(&mut *tx)
                .await?;
            let Some(slot_id) = sqlx::query_scalar::<_, u64>(
                "SELECT id FROM pet_villa_slots WHERE character_id=? AND pet_instance_id=? AND status=2 LIMIT 1 FOR UPDATE",
            )
            .bind(char_id)
            .bind(pet_instance_id)
            .fetch_optional(&mut *tx)
            .await?
            else {
                return Ok(error("Training slot not found."));
            };
            sqlx::query(
                "UPDATE pet_villa_slots SET status=1, pet_instance_id=NULL, training_ends_at=NULL, gold_spent=0, updated_at=NOW() WHERE id=?",
            )
            .bind(slot_id)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            Ok(json!({"status": 1, "result": "Training cancelled."}))
        }
        "checkoutPet" => {
            let pet_instance_id = integer(args, 2);
            let mut tx = pool.begin().await?;
            let owner_level: i32 =
                sqlx::query_scalar("SELECT level FROM characters WHERE id=? FOR UPDATE")
                    .bind(char_id)
                    .fetch_one(&mut *tx)
                    .await?;
            let Some(slot_id) = sqlx::query_scalar::<_, u64>(
                "SELECT id FROM pet_villa_slots WHERE character_id=? AND pet_instance_id=? AND status=3 LIMIT 1 FOR UPDATE",
            )
            .bind(char_id)
            .bind(pet_instance_id)
            .fetch_optional(&mut *tx)
            .await?
            else {
                return Ok(error("No completed training found for this pet."));
            };
            let pet_exists: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM character_pets WHERE id=? AND character_id=?",
            )
            .bind(pet_instance_id)
            .bind(char_id)
            .fetch_one(&mut *tx)
            .await?;
            if pet_exists == 0 {
                return Ok(error("Pet not found."));
            }
            sqlx::query(
                "UPDATE character_pets SET level=GREATEST(level,LEAST(level+1,?)), xp=0, updated_at=NOW() WHERE id=?",
            )
            .bind(owner_level.min(85))
            .bind(pet_instance_id)
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                "UPDATE pet_villa_slots SET status=1, pet_instance_id=NULL, training_ends_at=NULL, gold_spent=0, updated_at=NOW() WHERE id=?",
            )
            .bind(slot_id)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            Ok(json!({"status": 1}))
        }
        "unlockSlots" => {
            let payment_type = string(args, 2);
            let mut tx = pool.begin().await?;
            sqlx::query("SELECT id FROM characters WHERE id=? FOR UPDATE")
                .bind(char_id)
                .fetch_one(&mut *tx)
                .await?;
            let Some(slot_id) = sqlx::query_scalar::<_, u64>(
                "SELECT id FROM pet_villa_slots WHERE character_id=? AND status=0 ORDER BY slot_index LIMIT 1 FOR UPDATE",
            )
            .bind(char_id)
            .fetch_optional(&mut *tx)
            .await?
            else {
                return Ok(json!({"status": 2, "result": "All slots are already unlocked."}));
            };
            let Some(char_row) = sqlx::query(
                "SELECT c.user_id, u.tokens FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE",
            )
            .bind(char_id)
            .fetch_optional(&mut *tx)
            .await?
            else {
                return Ok(error("Character not found."));
            };

            if payment_type == "tokens" {
                let cost = 400;
                let tokens: i32 = char_row.try_get("tokens")?;
                if tokens < cost {
                    return Ok(json!({
                        "status": 2,
                        "result": format!("Not enough tokens. Unlocking requires {cost} tokens.")
                    }));
                }
                let user_id: u64 = char_row.try_get("user_id")?;
                sqlx::query("UPDATE users SET tokens=tokens-?, updated_at=NOW() WHERE id=?")
                    .bind(cost)
                    .bind(user_id)
                    .execute(&mut *tx)
                    .await?;
                sqlx::query("UPDATE pet_villa_slots SET status=1, updated_at=NOW() WHERE id=?")
                    .bind(slot_id)
                    .execute(&mut *tx)
                    .await?;
            } else if payment_type == "kunai" {
                let cost = 300;
                if !crate::inventory::take(&mut tx, char_id, "material_1002", cost).await? {
                    return Ok(json!({
                        "status": 2,
                        "result": format!("Not enough Friendship Kunai. Unlocking requires {cost}.")
                    }));
                }
                sqlx::query("UPDATE pet_villa_slots SET status=1, updated_at=NOW() WHERE id=?")
                    .bind(slot_id)
                    .execute(&mut *tx)
                    .await?;
            } else {
                return Ok(error("Invalid payment type."));
            }
            tx.commit().await?;
            Ok(json!({"status": 1}))
        }
        _ => Ok(error("Unknown pet villa operation")),
    }
}

// -----------------------------------------------------------------------------
// PetCombination.*
// -----------------------------------------------------------------------------

pub async fn dispatch_combination(
    state: &crate::GameState,
    pool: &MySqlPool,
    method: &str,
    args: &[Json],
) -> Result {
    let char_id = integer(args, 0);
    if let Some(err) = guard(pool, char_id, &string(args, 1), true).await? {
        return Ok(err);
    }
    match method {
        "getData" => {
            let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM characters WHERE id=?")
                .bind(char_id)
                .fetch_one(pool)
                .await?;
            if exists == 0 {
                return Ok(error("Character not found."));
            }
            let now = now_unix();
            let boosts = state.pet_boosts.lock().await;
            let remaining = boosts
                .get(&char_id)
                .map(|exp| (*exp - now).max(0))
                .unwrap_or(0);
            Ok(json!({
                "status": 1,
                "boost": if remaining > 0 { json!(remaining) } else { Json::Null }
            }))
        }
        "boostRate" => {
            let now = now_unix();
            let mut boosts = state.pet_boosts.lock().await;
            if boosts.get(&char_id).is_some_and(|exp| *exp > now) {
                return Ok(json!({
                    "status": 2,
                    "result": "Combine rate boost is already active."
                }));
            }
            let mut tx = pool.begin().await?;
            let Some(char_row) = sqlx::query(
                "SELECT c.user_id, u.tokens FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE",
            )
            .bind(char_id)
            .fetch_optional(&mut *tx)
            .await?
            else {
                return Ok(error("Character not found."));
            };
            let tokens: i32 = char_row.try_get("tokens")?;
            let boost_price = 500;
            if tokens < boost_price {
                return Ok(json!({
                    "status": 2,
                    "result": format!("Not enough tokens. Boost requires {boost_price} tokens.")
                }));
            }
            let user_id: u64 = char_row.try_get("user_id")?;
            sqlx::query("UPDATE users SET tokens=tokens-?, updated_at=NOW() WHERE id=?")
                .bind(boost_price)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            let boost_duration = 10800;
            boosts.insert(char_id, now + boost_duration);
            Ok(json!({
                "status": 1,
                "result": "Combine rate boosted for 3 hours! Good luck!",
                "boost": boost_duration
            }))
        }
        "combinePet" => {
            let left_id = integer(args, 2);
            let right_id = integer(args, 3);
            if left_id == right_id {
                return Ok(json!({
                    "status": 2,
                    "result": "Cannot combine a pet with itself."
                }));
            }
            let mut tx = pool.begin().await?;
            let Some(char_row) = sqlx::query("SELECT gold FROM characters WHERE id=? FOR UPDATE")
                .bind(char_id)
                .fetch_optional(&mut *tx)
                .await?
            else {
                return Ok(error("Character not found."));
            };
            let left_pet = sqlx::query(
                "SELECT id, level, maturity_points FROM character_pets WHERE id=? AND character_id=? FOR UPDATE",
            )
            .bind(left_id)
            .bind(char_id)
            .fetch_optional(&mut *tx)
            .await?;
            let right_pet = sqlx::query(
                "SELECT id, level, maturity_points FROM character_pets WHERE id=? AND character_id=? FOR UPDATE",
            )
            .bind(right_id)
            .bind(char_id)
            .fetch_optional(&mut *tx)
            .await?;
            let (Some(left_pet), Some(right_pet)) = (left_pet, right_pet) else {
                return Ok(json!({
                    "status": 2,
                    "result": "One or both pets were not found."
                }));
            };
            let left_lvl: i32 = left_pet.try_get("level")?;
            let right_lvl: i32 = right_pet.try_get("level")?;
            if left_lvl < 30 || right_lvl < 30 {
                return Ok(json!({
                    "status": 2,
                    "result": "Both pets must be level 30 or higher."
                }));
            }
            let left_mp: u32 = left_pet.try_get("maturity_points")?;
            let right_mp: u32 = right_pet.try_get("maturity_points")?;
            if left_mp < 100 || right_mp < 100 {
                return Ok(json!({
                    "status": 2,
                    "result": "Both pets need at least 100 Maturity Points (MP)."
                }));
            }
            let total_gold = 10000;
            let char_gold: i32 = char_row.try_get("gold")?;
            if char_gold < total_gold {
                return Ok(json!({
                    "status": 2,
                    "result": format!("Not enough gold. Combination requires {total_gold} gold.")
                }));
            }
            sqlx::query("UPDATE characters SET gold=gold-?, updated_at=NOW() WHERE id=?")
                .bind(total_gold)
                .bind(char_id)
                .execute(&mut *tx)
                .await?;

            let now = now_unix();
            let boosted = state
                .pet_boosts
                .lock()
                .await
                .get(&char_id)
                .is_some_and(|exp| *exp > now);
            let rate = if boosted { 80u32 } else { 50u32 };
            let mut bytes = [0u8; 4];
            let _ = getrandom::fill(&mut bytes);
            let roll = 1 + (u32::from_le_bytes(bytes) % 100);
            let success = roll <= rate;

            if success {
                sqlx::query("DELETE FROM character_pets WHERE id IN (?, ?)")
                    .bind(left_id)
                    .bind(right_id)
                    .execute(&mut *tx)
                    .await?;
                crate::rewards::give_item(&mut tx, char_id, "pet_combo_reward", 1).await?;
                tx.commit().await?;
                Ok(json!({
                    "status": 1,
                    "success": true,
                    "pets": "pet_combo_reward"
                }))
            } else {
                sqlx::query(
                    "UPDATE character_pets SET maturity_points=?, updated_at=NOW() WHERE id=?",
                )
                .bind(left_mp / 2)
                .bind(left_id)
                .execute(&mut *tx)
                .await?;
                sqlx::query(
                    "UPDATE character_pets SET maturity_points=?, updated_at=NOW() WHERE id=?",
                )
                .bind(right_mp / 2)
                .bind(right_id)
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
                Ok(json!({
                    "status": 1,
                    "success": false
                }))
            }
        }
        _ => Ok(error("Unknown PetCombination operation")),
    }
}
