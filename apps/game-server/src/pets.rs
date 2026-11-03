use crate::account::{guard, integer, string};
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};
type Result = std::result::Result<Json, sqlx::Error>;

pub async fn list_data(pool: &MySqlPool, id: i64) -> std::result::Result<Vec<Json>, sqlx::Error> {
    let rows=sqlx::query("SELECT p.*,c.skills,c.swf,c.name AS catalog_name FROM character_pets p LEFT JOIN pets c ON c.pet_id=p.pet_id JOIN characters ch ON ch.id=p.character_id WHERE p.character_id=? ORDER BY (p.id=ch.equipment_pet) DESC,p.id").bind(id).fetch_all(pool).await?;
    rows.iter().map(|r| {
        let level:i32=r.try_get("level")?;
        let skills=r.try_get::<Option<Json>,_>("skills")?.and_then(|v|v.as_array().cloned());
        let mut flags:Vec<_>=if let Some(skills)=skills.filter(|v|!v.is_empty()) {skills.iter().map(|s|if i64::from(level)>=s["level"].as_i64().unwrap_or(1) {"1"}else{"0"}).collect()}else{[1,10,20,30,40,50].iter().map(|required|if level>=*required {"1"}else{"0"}).collect()};
        flags.resize(flags.len().max(6),"0");
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
        sqlx::query("INSERT INTO character_pets (character_id,pet_id,name,level,xp,created_at,updated_at) VALUES (?,?,?,1,0,NOW(),NOW())").bind(id).bind(string(args,2)).bind(config.try_get::<String,_>("name")?).execute(&mut *tx).await?;
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
