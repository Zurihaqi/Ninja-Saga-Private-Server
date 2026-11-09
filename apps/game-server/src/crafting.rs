use crate::{
    account::{guard, integer, string},
    inventory,
};
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};
type Result = std::result::Result<Json, sqlx::Error>;
fn denied(message: &str) -> Json {
    json!({"status":2,"error":0,"result":message})
}

pub async fn forge(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(e) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(e);
    }
    let output = string(args, 2);
    let currency = string(args, 3);
    let recipes: Json =
        serde_json::from_str(include_str!("../../../database/catalogs/blacksmith.json"))
            .expect("checked-in recipes");
    let Some(recipe) = recipes.get(&output) else {
        return Ok(denied("Recipe not found"));
    };
    if !matches!(currency.as_str(), "gold" | "tokens") {
        return Ok(denied("Invalid currency"));
    }
    let cost = recipe[&currency].as_i64().unwrap_or(i64::MAX);
    let base = recipe["req_weapon"].as_str().unwrap_or("");
    let mut tx = pool.begin().await?;
    let char=sqlx::query("SELECT c.gold,c.user_id,c.equipment_weapon,u.tokens FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE").bind(id).fetch_one(&mut *tx).await?;
    if cost < 0 || i64::from(char.try_get::<i32, _>(&*currency)?) < cost {
        return Ok(denied("Not enough resources!"));
    }
    let mut consumed = vec![];
    let mut amounts = vec![];
    if currency == "gold"
        && let Some(mats) = recipe["mats"].as_object()
    {
        for (item, qty) in mats {
            let qty = qty.as_i64().unwrap_or(0);
            if !inventory::take(&mut tx, id, item, qty).await? {
                return Ok(denied(&format!("Not enough {item}!")));
            }
            consumed.push(item.to_owned());
            amounts.push(qty);
        }
    }
    if !base.is_empty() {
        if !inventory::take(&mut tx, id, base, 1).await? {
            return Ok(denied("You do not have the required weapon"));
        }
        if char
            .try_get::<Option<String>, _>("equipment_weapon")?
            .as_deref()
            == Some(base)
        {
            sqlx::query("UPDATE characters SET equipment_weapon='wpn_01' WHERE id=?")
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
        consumed.push(base.to_owned());
        amounts.push(1);
    }
    if currency == "gold" {
        sqlx::query("UPDATE characters SET gold=gold-?,updated_at=NOW() WHERE id=?")
            .bind(cost)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    } else {
        sqlx::query("UPDATE users SET tokens=tokens-?,updated_at=NOW() WHERE id=?")
            .bind(cost)
            .bind(char.try_get::<u64, _>("user_id")?)
            .execute(&mut *tx)
            .await?;
    }
    inventory::give(&mut tx, id, &output, 1).await?;
    tx.commit().await?;
    Ok(
        json!({"status":1,"error":0,"result":"Weapon Forged!","item":output,"requirements":[consumed,amounts]}),
    )
}
