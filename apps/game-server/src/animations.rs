//! Cosmetic animations use their own catalog and token prices, not equipment prices.
use crate::account::{guard, integer, string};
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};
type Result = std::result::Result<Json, sqlx::Error>;
fn denied(message: &str) -> Json {
    json!({"status":2,"result":message})
}
pub async fn dispatch(pool: &MySqlPool, args: &[Json], buy: bool) -> Result {
    let id = integer(args, 0);
    if let Some(e) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(e);
    }
    let animation = string(args, 2);
    let Some(config) = crate::economy::catalog_entry("animation", &animation) else {
        return Ok(json!({"status":0,"error":"Animation not found"}));
    };
    let price = config["price"].as_i64().unwrap_or(0);
    let mut tx = pool.begin().await?;
    let char=sqlx::query("SELECT c.user_id,u.tokens,u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE").bind(id).fetch_one(&mut *tx).await?;
    let owned:i64=sqlx::query_scalar("SELECT COUNT(*) FROM character_items WHERE character_id=? AND item_id=? AND category='animation' AND quantity>0").bind(id).bind(&animation).fetch_one(&mut *tx).await?;
    if !buy {
        return Ok(if owned > 0 || price == 0 {
            json!({"status":1})
        } else {
            denied("Animation not owned")
        });
    }
    if config["buyable"] != true || price <= 0 {
        return Ok(denied("Animation cannot be purchased"));
    }
    if config["premium"] == true && char.try_get::<i8, _>("account_type")? == 0 {
        return Ok(json!({"status":6,"result":"Upgrade premium to buy!"}));
    }
    if owned > 0 {
        return Ok(denied("Animation already owned"));
    }
    let tokens = i64::from(char.try_get::<i32, _>("tokens")?);
    if tokens < price {
        return Ok(denied("Not enough tokens!"));
    }
    sqlx::query("UPDATE users SET tokens=tokens-?,updated_at=NOW() WHERE id=?")
        .bind(price)
        .bind(char.try_get::<u64, _>("user_id")?)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO character_items (character_id,item_id,category,quantity,created_at,updated_at) VALUES (?,?,'animation',1,NOW(),NOW())").bind(id).bind(animation).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(json!({"status":1,"data":{"account_tokens":tokens-price}}))
}
