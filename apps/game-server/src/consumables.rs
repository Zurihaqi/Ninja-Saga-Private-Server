use crate::{
    account::{guard, integer, string},
    inventory,
};
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};
type Result = std::result::Result<Json, sqlx::Error>;
fn denied(message: &str) -> Json {
    json!({"status":2,"result":message})
}

pub async fn use_item(pool: &MySqlPool, args: &[Json], battle: bool) -> Result {
    let (id, key) = if battle {
        (integer(args, 1), string(args, 0))
    } else {
        (integer(args, 0), string(args, 1))
    };
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let item = string(args, 2);
    let Some(config) = crate::economy::catalog_entry("library", &item) else {
        return Ok(denied("Item data not found"));
    };
    if (battle && config["type"] != "item") || (!battle && config["usable"] != true) {
        return Ok(denied("Item cannot be used"));
    }
    let mut tx = pool.begin().await?;
    let char = sqlx::query("SELECT level,`rank`,gender FROM characters WHERE id=? FOR UPDATE")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    if i64::from(char.try_get::<i32, _>("level")?) < config["level"].as_i64().unwrap_or(0) {
        return Ok(denied("Level too low"));
    }
    let mut rewards = Vec::<String>::new();
    if !battle {
        let tp = match item.as_str() {
            "essential_03" => 5,
            "essential_04" => 10,
            "essential_05" => 20,
            "essential_06" => 50,
            "essential_07" => 100,
            "essential_08" => 200,
            _ => 0,
        };
        if tp > 0 {
            if char.try_get::<i32, _>("rank")? < 5 {
                return Ok(denied("Item cannot be used"));
            }
            sqlx::query("UPDATE characters SET tp=tp+? WHERE id=? AND tp<=2147483447")
                .bind(tp)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            rewards.push(format!("tp_{tp}"));
        } else {
            rewards = match item.as_str() {
                "essential_12" => vec![
                    "set_831_%s".into(),
                    "hair_354_%s".into(),
                    "wpn_664".into(),
                    "back_350".into(),
                ],
                "essential_13" => vec![
                    "set_2179_%s".into(),
                    "hair_2144_%s".into(),
                    "back_2192".into(),
                ],
                "essential_14" => {
                    let token = authentication::mission_token()
                        .map_err(|_| sqlx::Error::Protocol("Entropy unavailable".into()))?;
                    vec![format!(
                        "item_{}",
                        55 + usize::from(token.as_bytes()[0]) % 3
                    )]
                }
                _ => return Ok(denied("Item cannot be used")),
            };
            for reward in &rewards {
                inventory::give(
                    &mut tx,
                    id,
                    &reward.replace("%s", &char.try_get::<i8, _>("gender")?.to_string()),
                    1,
                )
                .await?;
            }
        }
    }
    if !inventory::take(&mut tx, id, &item, 1).await? {
        return Ok(denied("Item not found in inventory"));
    }
    tx.commit().await?;
    Ok(if battle {
        json!({"status":1,"result":"Item used"})
    } else {
        json!({"status":1,"result":"Item used","rewards":rewards})
    })
}
