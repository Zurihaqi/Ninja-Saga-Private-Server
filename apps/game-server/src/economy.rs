//! Normal shop and academy purchases, using the original catalog prices and response codes.
use crate::account::{guard, integer, string};
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};
type Result = std::result::Result<Json, sqlx::Error>;
type Tx<'a> = sqlx::Transaction<'a, sqlx::MySql>;
fn error(message: &str) -> Json {
    json!({"status":0,"error":message})
}

struct Wallet {
    gold: i64,
    tokens: i64,
    level: i32,
    account: i8,
    uid: u64,
    prestige: i32,
    pvp: i64,
    elements: [i32; 3],
}
async fn wallet(tx: &mut Tx<'_>, id: i64) -> std::result::Result<Wallet, sqlx::Error> {
    let r=sqlx::query("SELECT c.gold,c.level,c.user_id,c.prestige,c.pvp_points,c.element_1,c.element_2,c.element_3,u.tokens,u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE").bind(id).fetch_one(&mut **tx).await?;
    Ok(Wallet {
        gold: r.try_get::<i32, _>("gold")?.into(),
        tokens: r.try_get::<i32, _>("tokens")?.into(),
        level: r.try_get("level")?,
        account: r.try_get("account_type")?,
        uid: r.try_get("user_id")?,
        prestige: r.try_get("prestige")?,
        pvp: r.try_get::<i32, _>("pvp_points")?.into(),
        elements: [
            r.try_get("element_1")?,
            r.try_get("element_2")?,
            r.try_get("element_3")?,
        ],
    })
}
async fn save_wallet(tx: &mut Tx<'_>, id: i64, w: &Wallet) -> std::result::Result<(), sqlx::Error> {
    sqlx::query("UPDATE characters SET gold=?,pvp_points=?,element_1=?,element_2=?,element_3=?,updated_at=NOW() WHERE id=?").bind(w.gold).bind(w.pvp).bind(w.elements[0]).bind(w.elements[1]).bind(w.elements[2]).bind(id).execute(&mut **tx).await?;
    sqlx::query("UPDATE users SET tokens=?,updated_at=NOW() WHERE id=?")
        .bind(w.tokens)
        .bind(w.uid)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
pub(crate) fn catalog_entry(file: &str, id: &str) -> Option<Json> {
    let root = std::env::var("GAME_DATA_ROOT").ok()?;
    let bytes = std::fs::read(std::path::Path::new(&root).join(format!("{file}.json"))).ok()?;
    let values: Json = serde_json::from_slice(&bytes).ok()?;
    values.as_array()?.iter().find(|v| v["id"] == id).cloned()
}
fn total(price: i64, quantity: i64) -> Option<i64> {
    price
        .checked_mul(quantity)
        .filter(|n| *n >= 0 && *n <= i64::from(i32::MAX))
}

pub async fn buy_essential(pool: &MySqlPool, args: &[Json], item: &str, field: &str) -> Result {
    let id = integer(args, 1);
    if let Some(e) = guard(pool, id, &string(args, 0), true).await? {
        return Ok(e);
    }
    let qty = integer(args, 2);
    if !(1..=1_000_000).contains(&qty) {
        return Ok(error("Invalid quantity"));
    }
    let mut tx = pool.begin().await?;
    let mut w = wallet(&mut tx, id).await?;
    let Some(price) =
        sqlx::query_scalar::<_, i32>("SELECT price_tokens FROM items WHERE item_id=?")
            .bind(item)
            .fetch_optional(&mut *tx)
            .await?
    else {
        return Ok(error("Item not found"));
    };
    let Some(cost) = total(i64::from(price), qty) else {
        return Ok(error("Invalid price"));
    };
    if w.tokens < cost {
        return Ok(json!({"status":2,"result":"Not enough tokens!"}));
    }
    w.tokens -= cost;
    crate::inventory::give(&mut tx, id, item, qty).await?;
    save_wallet(&mut tx, id, &w).await?;
    let quantity: i32 = sqlx::query_scalar(
        "SELECT quantity FROM character_items WHERE character_id=? AND item_id=? LIMIT 1",
    )
    .bind(id)
    .bind(item)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    let mut result = json!({"status":1});
    result[field] = json!(quantity);
    Ok(result)
}

pub async fn buy_item(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let item = string(args, 2);
    let quantity = integer(args, 3);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    if !(1..=1_000_000).contains(&quantity) {
        return Ok(error("Invalid quantity"));
    }
    if item.starts_with("skill_") {
        return buy_skill(pool, &[json!(key), json!(id), json!(item)]).await;
    }
    let mut tx = pool.begin().await?;
    let mut w = wallet(&mut tx, id).await?;
    let Some(config) = sqlx::query("SELECT * FROM items WHERE item_id=?")
        .bind(&item)
        .fetch_optional(&mut *tx)
        .await?
    else {
        return Ok(error("Item data not found!"));
    };
    if config.try_get::<bool, _>("premium")? && w.account == 0 {
        return Ok(json!({"status":6,"result":"Upgrade premium to buy!"}));
    }
    if w.level < config.try_get::<i32, _>("level")? {
        return Ok(json!({"status":5,"result":"Level too low!"}));
    }
    let price_pvp = catalog_entry("library", &item)
        .and_then(|v| v["price_pvp"].as_i64())
        .unwrap_or(0);
    let Some(gold) = total(config.try_get::<i32, _>("price_gold")?.into(), quantity) else {
        return Ok(error("Invalid price"));
    };
    let Some(tokens) = total(config.try_get::<i32, _>("price_tokens")?.into(), quantity) else {
        return Ok(error("Invalid price"));
    };
    let Some(pvp) = total(price_pvp, quantity) else {
        return Ok(error("Invalid price"));
    };
    if w.gold < gold || w.tokens < tokens {
        return Ok(json!({"status":3,"result":"Not enough resources!"}));
    }
    if w.pvp < pvp {
        return Ok(json!({"status":3,"result":"Not enough PvP points!"}));
    }
    let existing=sqlx::query("SELECT id,quantity FROM character_items WHERE character_id=? AND item_id=? LIMIT 1 FOR UPDATE").bind(id).bind(&item).fetch_optional(&mut *tx).await?;
    if let Some(row) = existing {
        let Some(new_quantity) = quantity
            .checked_add(row.try_get::<i32, _>("quantity")?.into())
            .filter(|v| *v <= i64::from(i32::MAX))
        else {
            return Ok(error("Inventory quantity limit exceeded"));
        };
        sqlx::query("UPDATE character_items SET quantity=?,updated_at=NOW() WHERE id=?")
            .bind(new_quantity)
            .bind(row.try_get::<u64, _>("id")?)
            .execute(&mut *tx)
            .await?;
    } else {
        sqlx::query("INSERT INTO character_items(character_id,item_id,quantity,category,created_at,updated_at) VALUES(?,?,?,?,NOW(),NOW())").bind(id).bind(&item).bind(quantity).bind(config.try_get::<String,_>("category")?).execute(&mut *tx).await?;
    }
    w.gold -= gold;
    w.tokens -= tokens;
    w.pvp -= pvp;
    save_wallet(&mut tx, id, &w).await?;
    tx.commit().await?;
    Ok(
        json!({"status":1,"error":0,"data":{"character_gold":w.gold,"character_prestige":w.prestige,"account_tokens":w.tokens,"pvp_points":w.pvp}}),
    )
}

pub async fn sell_item(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let item = string(args, 2);
    let quantity = integer(args, 3);
    if let Some(e) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(e);
    }
    if !(1..=1_000_000).contains(&quantity) {
        return Ok(error("Invalid quantity"));
    }
    let mut tx = pool.begin().await?;
    let mut w = wallet(&mut tx, id).await?;
    let Some(owned)=sqlx::query("SELECT id,quantity FROM character_items WHERE character_id=? AND item_id=? LIMIT 1 FOR UPDATE").bind(id).bind(&item).fetch_optional(&mut *tx).await? else{return Ok(error("Not enough items to sell!"));};
    let remaining = i64::from(owned.try_get::<i32, _>("quantity")?) - quantity;
    if remaining < 0 {
        return Ok(error("Not enough items to sell!"));
    }
    let Some(price) = sqlx::query_scalar::<_, i32>("SELECT price_gold FROM items WHERE item_id=?")
        .bind(&item)
        .fetch_optional(&mut *tx)
        .await?
    else {
        return Ok(error("Item data not found!"));
    };
    let sell_price = if price > 0 {
        i64::from(price) / 2
    } else {
        let entry = catalog_entry("library", &item).unwrap_or(Json::Null);
        if entry["sellable"] == false {
            return Ok(error("Item cannot be sold!"));
        }
        entry["sell_price"].as_i64().unwrap_or(0)
    };
    if sell_price <= 0 {
        return Ok(error("Item cannot be sold!"));
    }
    let Some(proceeds) = total(sell_price, quantity) else {
        return Ok(error("Invalid price"));
    };
    let Some(gold) = w
        .gold
        .checked_add(proceeds)
        .filter(|n| *n <= i64::from(i32::MAX))
    else {
        return Ok(error("Currency limit exceeded"));
    };
    w.gold = gold;
    save_wallet(&mut tx, id, &w).await?;
    if remaining == 0 {
        sqlx::query("DELETE FROM character_items WHERE id=?")
            .bind(owned.try_get::<u64, _>("id")?)
            .execute(&mut *tx)
            .await?;
    } else {
        sqlx::query("UPDATE character_items SET quantity=?,updated_at=NOW() WHERE id=?")
            .bind(remaining)
            .bind(owned.try_get::<u64, _>("id")?)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(json!({"status":1,"error":0,"data":{"character_gold":gold}}))
}

pub async fn buy_skill(pool: &MySqlPool, args: &[Json]) -> Result {
    let key = string(args, 0);
    let id = integer(args, 1);
    let skill = string(args, 2);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let mut tx = pool.begin().await?;
    let mut w = wallet(&mut tx, id).await?;
    let learned: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM character_skills WHERE character_id=? AND skill_id=?",
    )
    .bind(id)
    .bind(&skill)
    .fetch_one(&mut *tx)
    .await?;
    if learned > 0 {
        return Ok(json!({"status":2}));
    }
    let Some(config) = sqlx::query("SELECT * FROM skills WHERE skill_id=?")
        .bind(&skill)
        .fetch_optional(&mut *tx)
        .await?
    else {
        return Ok(error("Skill data not found!"));
    };
    if config.try_get::<bool, _>("premium")? && w.account == 0 {
        return Ok(json!({"status":6}));
    }
    if w.level < config.try_get::<i32, _>("level")? {
        return Ok(json!({"status":5}));
    }
    let element = i32::from(config.try_get::<i8, _>("element")?);
    if (1..=5).contains(&element) && !w.elements.contains(&element) {
        let max = if w.account >= 1 { 3 } else { 2 };
        if w.elements.iter().filter(|n| **n > 0).count() >= max {
            return Ok(json!({"status":4}));
        }
        if let Some(slot) = w.elements.iter_mut().find(|n| **n == 0) {
            *slot = element;
        }
    }
    let gold = i64::from(config.try_get::<i32, _>("price_gold")?);
    let tokens = i64::from(config.try_get::<i32, _>("price_tokens")?);
    if gold < 0 || tokens < 0 {
        return Ok(error("Invalid price"));
    }
    if w.gold < gold || w.tokens < tokens {
        return Ok(json!({"status":3}));
    }
    w.gold -= gold;
    w.tokens -= tokens;
    save_wallet(&mut tx, id, &w).await?;
    sqlx::query("INSERT INTO character_skills(character_id,skill_id,created_at,updated_at) VALUES(?,?,NOW(),NOW())").bind(id).bind(&skill).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(
        json!({"status":1,"data":{"character_gold":w.gold,"account_tokens":w.tokens,"character_element_1":w.elements[0],"character_element_2":w.elements[1],"character_element_3":w.elements[2]}}),
    )
}
