//! Former paid products are free local claims. No payment gateway or receipt is used.
use crate::{
    account::{guard, integer, string},
    rewards,
};
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};
type Result = std::result::Result<Json, sqlx::Error>;
pub async fn claim(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(e) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(e);
    }
    let product = string(args, 2);
    let request = string(args, 3);
    if request.len() < 8
        || request.len() > 128
        || !request
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
    {
        return Ok(json!({"status":2,"result":"Invalid free claim request."}));
    }
    let mut grant = vec![];
    let mut premium = false;
    if let Some(amount) = product
        .strip_prefix("id.ninjasage.tokens.")
        .and_then(|s| s.parse::<i32>().ok())
    {
        if ![500, 1000, 2500, 5000, 25000, 50000].contains(&amount) {
            return Ok(json!({"status":2,"result":"Unknown free product."}));
        }
        grant.push(format!("tokens_{amount}"));
    } else if matches!(
        product.as_str(),
        "id.ninjasage.emblem" | "id.ninjasage.trial.emblem"
    ) {
        premium = true;
    } else if product == "id.ninjasage.ancestor" {
        grant = vec![
            "hair_309_%s".into(),
            "set_1155_%s".into(),
            "back_605".into(),
            "wpn_1341".into(),
        ];
    } else if let Some(index) = product
        .strip_prefix("id.ninjasage.elementalars.")
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|v| (1..=4).contains(v))
    {
        let cfg:Option<Json>=sqlx::query_scalar("SELECT data FROM game_events WHERE type='package' AND active=1 ORDER BY sort_order,id LIMIT 1").fetch_optional(pool).await?;
        let compiled: Json =
            serde_json::from_str(include_str!("../../../database/catalogs/events.json"))
                .expect("compiled event catalog");
        let fallback = compiled
            .as_array()
            .and_then(|a| a.iter().find(|e| e["type"] == "package"))
            .and_then(|e| e.get("data"));
        let Some(package) = cfg
            .as_ref()
            .or(fallback)
            .and_then(|v| v["content"].as_array())
            .and_then(|a| a.get(index - 1))
        else {
            return Ok(json!({"status":2,"result":"Free package not found."}));
        };
        for reward in package["rewards"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Json::as_str)
        {
            if reward == "emblem" {
                premium = true;
            } else {
                grant.push(reward.to_owned());
            }
        }
    } else {
        return Ok(json!({"status":2,"result":"Unknown free product."}));
    }
    let mut tx = pool.begin().await?;
    let c=sqlx::query("SELECT c.user_id,c.gender,c.gold,u.tokens,u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE").bind(id).fetch_one(&mut *tx).await?;
    let uid = c.try_get::<u64, _>("user_id")?;
    let exists: Option<String> = sqlx::query_scalar(
        "SELECT product_id FROM free_product_claims WHERE user_id=? AND request_id=?",
    )
    .bind(uid)
    .bind(&request)
    .fetch_optional(&mut *tx)
    .await?;
    if exists.as_ref().is_some_and(|p| p != &product) {
        return Ok(json!({"status":2,"result":"This request belongs to another product."}));
    }
    let repeated = exists.is_some();
    let mut granted = vec![];
    if !repeated {
        // Clamp free currency claims at the schema's balance limit, rather than overflow.
        let tokens: i32 = c.try_get("tokens")?;
        let total_tokens: i64 = grant
            .iter()
            .filter_map(|s| s.strip_prefix("tokens_"))
            .filter_map(|s| s.parse::<i64>().ok())
            .sum();
        if i64::from(tokens) + total_tokens > i64::from(i32::MAX) {
            return Ok(json!({"status":2,"result":"Token balance is already at its limit."}));
        }
        for reward in grant {
            rewards::grant(&mut tx, id, &reward).await?;
            granted.push(reward.replace("%s", &c.try_get::<i8, _>("gender")?.to_string()));
        }
        if premium {
            if c.try_get::<i8, _>("account_type")? == 0 {
                crate::inventory::give(&mut tx, id, "essential_10", 1).await?;
                granted.push("essential_10".into());
            }
            sqlx::query("UPDATE users SET account_type=IF(account_type=0,1,account_type),updated_at=NOW() WHERE id=?").bind(uid).execute(&mut *tx).await?;
        }
        sqlx::query("INSERT INTO free_product_claims(user_id,character_id,request_id,product_id) VALUES(?,?,?,?)").bind(uid).bind(id).bind(request).bind(product).execute(&mut *tx).await?;
    }
    let u = sqlx::query("SELECT tokens,account_type FROM users WHERE id=?")
        .bind(uid)
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(
        json!({"status":1,"result":if repeated {"This free claim was already delivered."} else {"Free product claimed. No payment is required."},"rewards":granted,"tokens":u.try_get::<i32,_>("tokens")?,"account_type":i32::from(u.try_get::<i8,_>("account_type")?>0),"emblem_duration":-1}),
    )
}
