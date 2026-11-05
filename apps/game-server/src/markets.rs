//! Material Market, Mysterious Market, Special Deals, and Exotic Package AMF services.
//! Source:
//! - `app/Services/Amf/MaterialMarketService.php`
//! - `app/Services/Amf/MysteriousMarketService.php`
//! - `app/Services/Amf/SpecialDealsService.php`
//! - `app/Services/Amf/ExoticPackageService.php`
use crate::{
    account::{guard, integer, string},
    inventory, rewards,
};
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};

type Result = std::result::Result<Json, sqlx::Error>;

fn error(message: &str) -> Json {
    json!({"status":0,"error":message})
}

async fn game_config(
    pool: &MySqlPool,
    key: &str,
) -> std::result::Result<Option<Json>, sqlx::Error> {
    let raw: Option<String> =
        sqlx::query_scalar("SELECT CAST(value AS CHAR) FROM game_configs WHERE `key`=? LIMIT 1")
            .bind(key)
            .fetch_optional(pool)
            .await?;
    Ok(raw.and_then(|s| serde_json::from_str(&s).ok()))
}

// -----------------------------------------------------------------------------
// MaterialMarket.*
// -----------------------------------------------------------------------------

pub async fn dispatch_material_market(pool: &MySqlPool, method: &str, args: &[Json]) -> Result {
    match method {
        "getItems" => material_market_get_items(pool, args).await,
        "forgeItem" => material_market_forge_item(pool, args).await,
        _ => Ok(error("Unknown MaterialMarket operation")),
    }
}

fn load_material_market_recipes() -> Vec<(String, Vec<String>, Vec<i64>, Json)> {
    let catalog: Json = serde_json::from_str(include_str!(
        "../../../database/catalogs/material_market.json"
    ))
    .expect("checked-in material_market.json");
    let mut out = Vec::new();
    if let Some(arr) = catalog.as_array() {
        for entry in arr {
            if let Some(item) = entry["item"].as_str() {
                let mats = entry["materials"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(Json::as_str)
                            .map(str::to_owned)
                            .collect()
                    })
                    .unwrap_or_default();
                let qtys = entry["qty"]
                    .as_array()
                    .map(|a| a.iter().filter_map(Json::as_i64).collect())
                    .unwrap_or_default();
                let end = entry.get("end").cloned().unwrap_or(Json::Null);
                out.push((item.to_owned(), mats, qtys, end));
            }
        }
    }
    out
}

async fn material_market_get_items(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let recipes = load_material_market_recipes();
    let items: Vec<Json> = recipes
        .into_iter()
        .map(|(item, materials, qty, end)| {
            json!({
                "item": item,
                "requirements": {
                    "materials": materials,
                    "qty": qty
                },
                "end": end
            })
        })
        .collect();
    Ok(json!({
        "status": 1,
        "items": items
    }))
}

async fn material_market_forge_item(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let item_id = string(args, 2);
    let recipes = load_material_market_recipes();
    let Some((_, materials, qtys, _)) = recipes.into_iter().find(|(k, _, _, _)| *k == item_id)
    else {
        return Ok(json!({"status":2,"result":"Invalid item or recipe not found."}));
    };

    let mut tx = pool.begin().await?;
    for (idx, mat_id) in materials.iter().enumerate() {
        let required = qtys.get(idx).copied().unwrap_or(1).max(1);
        let held: i32 = sqlx::query_scalar(
            "SELECT quantity FROM character_items WHERE character_id=? AND item_id=? LIMIT 1 FOR UPDATE",
        )
        .bind(id)
        .bind(mat_id)
        .fetch_optional(&mut *tx)
        .await?
        .unwrap_or(0);
        if i64::from(held) < required {
            return Ok(json!({
                "status": 2,
                "result": format!("Not enough {mat_id} (have {held}, need {required}).")
            }));
        }
    }
    for (idx, mat_id) in materials.iter().enumerate() {
        let required = qtys.get(idx).copied().unwrap_or(1).max(1);
        if !inventory::take(&mut tx, id, mat_id, required).await? {
            return Ok(json!({
                "status": 2,
                "result": format!("Not enough {mat_id}.")
            }));
        }
    }
    rewards::give_item(&mut tx, id, &item_id, 1).await?;
    tx.commit().await?;

    Ok(json!({
        "status": 1,
        "item": item_id,
        "requirements": [materials, qtys]
    }))
}

// -----------------------------------------------------------------------------
// MysteriousMarket.*
// -----------------------------------------------------------------------------

pub async fn dispatch_mysterious_market(pool: &MySqlPool, method: &str, args: &[Json]) -> Result {
    match method {
        "getPackageData" => mysterious_get_package_data(pool, args).await,
        "buyPackage" => mysterious_buy_package(pool, args).await,
        "getAllPackagesList" => mysterious_get_all_packages_list(pool, args).await,
        "refreshPackage" => mysterious_refresh_package(pool, args).await,
        _ => Ok(error("Unknown MysteriousMarket operation")),
    }
}

struct ActiveMarket {
    id: u64,
    seconds_remaining: i64,
    discount: String,
    refresh_cost: i64,
    refresh_max: i64,
    items: Vec<Json>,
    all_packages: Vec<Json>,
}

async fn load_active_market(
    pool: &MySqlPool,
) -> std::result::Result<Option<ActiveMarket>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id, discount, refresh_cost, refresh_max, CAST(items AS CHAR) AS items_json, CAST(all_packages AS CHAR) AS all_json, GREATEST(0, TIMESTAMPDIFF(SECOND, NOW(), ends_at)) AS rem FROM mysterious_markets WHERE active=1 LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let items_raw: Option<String> = row.try_get("items_json")?;
    let all_raw: Option<String> = row.try_get("all_json")?;
    let items = items_raw
        .and_then(|s| serde_json::from_str::<Json>(&s).ok())
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    let all_packages = all_raw
        .and_then(|s| serde_json::from_str::<Json>(&s).ok())
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();

    Ok(Some(ActiveMarket {
        id: row.try_get("id")?,
        seconds_remaining: row.try_get::<i64, _>("rem").unwrap_or(0),
        discount: row.try_get("discount")?,
        refresh_cost: i64::from(row.try_get::<u32, _>("refresh_cost")?),
        refresh_max: i64::from(row.try_get::<u32, _>("refresh_max")?),
        items,
        all_packages,
    }))
}

async fn get_or_create_char_market_state(
    pool: &MySqlPool,
    char_id: i64,
    market: &ActiveMarket,
) -> std::result::Result<(u64, i64, Option<Vec<Json>>), sqlx::Error> {
    sqlx::query(
        "INSERT IGNORE INTO character_mysterious_markets (character_id, market_id, refresh_count, custom_items, created_at, updated_at) VALUES (?, ?, ?, NULL, NOW(), NOW())",
    )
    .bind(char_id)
    .bind(market.id)
    .bind(market.refresh_max)
    .execute(pool)
    .await?;
    let row = sqlx::query(
        "SELECT id, refresh_count, CAST(custom_items AS CHAR) AS custom_json FROM character_mysterious_markets WHERE character_id=? AND market_id=? LIMIT 1",
    )
    .bind(char_id)
    .bind(market.id)
    .fetch_one(pool)
    .await?;
    let custom_raw: Option<String> = row.try_get("custom_json")?;
    let custom_items = custom_raw
        .and_then(|s| serde_json::from_str::<Json>(&s).ok())
        .and_then(|v| v.as_array().cloned());
    Ok((
        row.try_get("id")?,
        i64::from(row.try_get::<u32, _>("refresh_count")?),
        custom_items,
    ))
}

async fn mysterious_get_package_data(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let Some(market) = load_active_market(pool).await? else {
        return Ok(json!({
            "status": 1,
            "end_time": 0,
            "discounts": "0",
            "refresh_cost": 0,
            "refresh_count": 0,
            "items": []
        }));
    };
    if market.seconds_remaining <= 0 {
        return Ok(json!({
            "status": 1,
            "end_time": 0,
            "discounts": "0",
            "refresh_cost": 0,
            "refresh_count": 0,
            "items": []
        }));
    }
    let (_, refresh_count, custom_items) =
        get_or_create_char_market_state(pool, id, &market).await?;
    let items = custom_items.unwrap_or(market.items);
    Ok(json!({
        "status": 1,
        "end_time": market.seconds_remaining,
        "discounts": market.discount,
        "refresh_cost": market.refresh_cost,
        "refresh_count": refresh_count,
        "items": items
    }))
}

async fn mysterious_buy_package(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let skill_id = string(args, 2);
    let Some(market) = load_active_market(pool).await? else {
        return Ok(json!({"status":2,"result":"The Mysterious Market is not currently open."}));
    };
    if market.seconds_remaining <= 0 {
        return Ok(json!({"status":2,"result":"The Mysterious Market is not currently open."}));
    }
    let (_, _, custom_items) = get_or_create_char_market_state(pool, id, &market).await?;
    let items = custom_items.unwrap_or(market.items);

    let Some((item_index, item_entry)) = items
        .iter()
        .enumerate()
        .find(|(_, entry)| entry["code"].as_str() == Some(skill_id.as_str()))
    else {
        return Ok(
            json!({"status":2,"result":"That skill is not available in the current market."}),
        );
    };

    let mut tx = pool.begin().await?;
    let Some(char_row) = sqlx::query(
        "SELECT c.user_id, u.tokens, u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(error("Character not found"));
    };

    let already_owned: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM character_skills WHERE character_id=? AND skill_id=?",
    )
    .bind(id)
    .bind(&skill_id)
    .fetch_one(&mut *tx)
    .await?;
    if already_owned > 0 {
        return Ok(json!({"status":2,"result":"You already own this skill."}));
    }

    let mut prev_code_to_remove: Option<String> = None;
    if item_index > 0
        && let Some(prev_code) = items[item_index - 1]["code"]
            .as_str()
            .filter(|s| !s.is_empty())
    {
        let owns_prev: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM character_skills WHERE character_id=? AND skill_id=?",
        )
        .bind(id)
        .bind(prev_code)
        .fetch_one(&mut *tx)
        .await?;
        if owns_prev == 0 {
            return Ok(json!({"status":2,"result":"You must purchase the previous skill first."}));
        }
        prev_code_to_remove = Some(prev_code.to_owned());
    }

    let account_type: i8 = char_row.try_get("account_type")?;
    let prices = item_entry["prices"].as_array();
    let price = if account_type == 1 {
        prices
            .and_then(|p| p.get(1))
            .and_then(Json::as_i64)
            .unwrap_or(0)
    } else {
        prices
            .and_then(|p| p.first())
            .and_then(Json::as_i64)
            .unwrap_or(0)
    };
    let tokens = i64::from(char_row.try_get::<i32, _>("tokens")?);
    if tokens < price {
        return Ok(json!({"status":2,"result":"Not enough tokens."}));
    }

    let user_id: u64 = char_row.try_get("user_id")?;
    sqlx::query("UPDATE users SET tokens=tokens-?,updated_at=NOW() WHERE id=?")
        .bind(price)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO character_skills (character_id,skill_id,created_at,updated_at) VALUES (?,?,NOW(),NOW())",
    )
    .bind(id)
    .bind(&skill_id)
    .execute(&mut *tx)
    .await?;
    if let Some(prev_code) = prev_code_to_remove {
        sqlx::query("DELETE FROM character_skills WHERE character_id=? AND skill_id=?")
            .bind(id)
            .bind(prev_code)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;

    Ok(json!({"status": 1}))
}

async fn mysterious_get_all_packages_list(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let Some(market) = load_active_market(pool).await? else {
        return Ok(json!({"status":2,"result":"No active market found."}));
    };
    let owned_rows = sqlx::query("SELECT skill_id FROM character_skills WHERE character_id=?")
        .bind(id)
        .fetch_all(pool)
        .await?;
    let owned_set: std::collections::HashSet<String> = owned_rows
        .iter()
        .filter_map(|r| r.try_get::<String, _>("skill_id").ok())
        .collect();

    let packages: Vec<Json> = market
        .all_packages
        .iter()
        .map(|entry| {
            let code = entry["code"].as_str().unwrap_or("");
            json!({
                "advanced_skill": code,
                "owned": owned_set.contains(code)
            })
        })
        .collect();

    Ok(json!({
        "status": 1,
        "packages": packages
    }))
}

async fn mysterious_refresh_package(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let Some(market) = load_active_market(pool).await? else {
        return Ok(json!({"status":2,"result":"The Mysterious Market is not currently open."}));
    };
    if market.seconds_remaining <= 0 {
        return Ok(json!({"status":2,"result":"The Mysterious Market is not currently open."}));
    }
    let (state_id, refresh_count, _) = get_or_create_char_market_state(pool, id, &market).await?;
    if refresh_count <= 0 {
        return Ok(json!({"status":2,"result":"You have no refreshes remaining."}));
    }
    if market.all_packages.is_empty() {
        return Ok(json!({"status":2,"result":"No packages available to refresh."}));
    }

    let mut tx = pool.begin().await?;
    let Some(char_row) = sqlx::query(
        "SELECT c.user_id, u.tokens FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(error("Character not found"));
    };
    let tokens = i64::from(char_row.try_get::<i32, _>("tokens")?);
    if tokens < market.refresh_cost {
        return Ok(json!({"status":2,"result":"Not enough tokens to refresh."}));
    }

    let owned_rows = sqlx::query("SELECT skill_id FROM character_skills WHERE character_id=?")
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
    let owned_set: std::collections::HashSet<String> = owned_rows
        .iter()
        .filter_map(|r| r.try_get::<String, _>("skill_id").ok())
        .collect();

    let mut available: Vec<Json> = market
        .all_packages
        .iter()
        .filter(|e| !owned_set.contains(e["code"].as_str().unwrap_or("")))
        .cloned()
        .collect();

    // Fisher-Yates shuffle using getrandom
    for i in (1..available.len()).rev() {
        let mut bytes = [0u8; 4];
        let _ = getrandom::fill(&mut bytes);
        let j = (u32::from_le_bytes(bytes) as usize) % (i + 1);
        available.swap(i, j);
    }

    let display_count = market.items.len().clamp(1, 4);
    let new_items: Vec<Json> = available.into_iter().take(display_count).collect();
    let new_items_json = serde_json::to_string(&new_items).unwrap_or_else(|_| "[]".into());

    let user_id: u64 = char_row.try_get("user_id")?;
    sqlx::query("UPDATE users SET tokens=tokens-?,updated_at=NOW() WHERE id=?")
        .bind(market.refresh_cost)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE character_mysterious_markets SET refresh_count=refresh_count-1, custom_items=CAST(? AS JSON), updated_at=NOW() WHERE id=?",
    )
    .bind(new_items_json)
    .bind(state_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(json!({"status": 1}))
}

// -----------------------------------------------------------------------------
// SpecialDeals.*
// -----------------------------------------------------------------------------

pub async fn dispatch_special_deals(pool: &MySqlPool, method: &str, args: &[Json]) -> Result {
    match method {
        "getDeals" => special_deals_get(pool, args).await,
        "buy" => special_deals_buy(pool, args).await,
        _ => Ok(error("Unknown SpecialDeals operation")),
    }
}

async fn special_deals_get(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let deals = game_config(pool, "special_deals")
        .await?
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    Ok(json!({
        "status": 1,
        "deals": deals
    }))
}

async fn special_deals_buy(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let deal_id = integer(args, 2);
    let deals = game_config(pool, "special_deals")
        .await?
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    let Some(deal) = deals
        .into_iter()
        .find(|d| d["id"].as_i64().unwrap_or(-1) == deal_id)
    else {
        return Ok(json!({"status":2,"result":"Deal not found."}));
    };
    let price = deal["price"].as_i64().unwrap_or(0);
    let reward_items: Vec<String> = deal["items"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(Json::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();

    let mut tx = pool.begin().await?;
    let Some(char_row) = sqlx::query(
        "SELECT c.user_id, u.tokens FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(error("Character not found."));
    };
    let tokens = i64::from(char_row.try_get::<i32, _>("tokens")?);
    if tokens < price {
        return Ok(json!({"status":2,"result":"Not enough tokens!"}));
    }
    let user_id: u64 = char_row.try_get("user_id")?;
    sqlx::query("UPDATE users SET tokens=tokens-?,updated_at=NOW() WHERE id=?")
        .bind(price)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    for entry in &reward_items {
        grant_deal_or_package_item(&mut tx, id, entry).await?;
    }
    tx.commit().await?;

    Ok(json!({
        "status": 1,
        "rewards": reward_items
    }))
}

async fn grant_deal_or_package_item(
    tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
    char_id: i64,
    entry: &str,
) -> std::result::Result<(), sqlx::Error> {
    let (item_id, qty_str) = entry
        .split_once(':')
        .map_or((entry, None), |(i, q)| (i, Some(q)));
    let qty = qty_str
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(1)
        .max(1);
    let prefix = item_id.split('_').next().unwrap_or("");
    let category = match prefix {
        "wpn" => "weapon".to_owned(),
        "back" => "back".to_owned(),
        "set" => "set".to_owned(),
        "accessory" => "accessory".to_owned(),
        "hair" => "hair".to_owned(),
        "skill" => "skill".to_owned(),
        "pet" => "pet".to_owned(),
        "material" => "material".to_owned(),
        "essential" => "essential".to_owned(),
        "item" => "item".to_owned(),
        _ => rewards::resolve_category(tx, item_id).await?,
    };
    let row = sqlx::query(
        "SELECT id,quantity FROM character_items WHERE character_id=? AND item_id=? LIMIT 1 FOR UPDATE",
    )
    .bind(char_id)
    .bind(item_id)
    .fetch_optional(&mut **tx)
    .await?;
    if let Some(row) = row {
        let new_qty = i64::from(row.try_get::<i32, _>("quantity")?)
            .saturating_add(qty)
            .min(i64::from(i32::MAX));
        sqlx::query("UPDATE character_items SET quantity=?,updated_at=NOW() WHERE id=?")
            .bind(new_qty)
            .bind(row.try_get::<u64, _>("id")?)
            .execute(&mut **tx)
            .await?;
    } else {
        sqlx::query(
            "INSERT INTO character_items (character_id,item_id,category,quantity,created_at,updated_at) VALUES (?,?,?,?,NOW(),NOW())",
        )
        .bind(char_id)
        .bind(item_id)
        .bind(category)
        .bind(qty)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// ExoticPackage.*
// -----------------------------------------------------------------------------

pub async fn dispatch_exotic_package(pool: &MySqlPool, method: &str, args: &[Json]) -> Result {
    match method {
        "get" => exotic_package_get(pool, args).await,
        "buy" => exotic_package_buy(pool, args).await,
        _ => Ok(error("Unknown ExoticPackage operation")),
    }
}

async fn exotic_package_get(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let rows = game_config(pool, "exotic_packages")
        .await?
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    let mut packages = serde_json::Map::new();
    for row in rows {
        let Some(key) = row["key"].as_str().filter(|s| !s.is_empty()) else {
            continue;
        };
        let name = row["name"].as_str().unwrap_or("");
        let price = row["price"].as_i64().unwrap_or(0);
        let items = row["items"].as_array().cloned().unwrap_or_default();
        packages.insert(
            key.to_owned(),
            json!({
                "name": name,
                "price": price,
                "items": items
            }),
        );
    }
    Ok(json!({
        "status": 1,
        "error": 0,
        "packages": Json::Object(packages)
    }))
}

async fn exotic_package_buy(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(err) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(err);
    }
    let package_key = string(args, 2);
    let rows = game_config(pool, "exotic_packages")
        .await?
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    let Some(package) = rows
        .into_iter()
        .find(|r| r["key"].as_str() == Some(package_key.as_str()))
    else {
        return Ok(json!({"status":2,"result":"Package not found."}));
    };
    let price = package["price"].as_i64().unwrap_or(0);
    let reward_items: Vec<String> = package["items"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(Json::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();

    let mut tx = pool.begin().await?;
    let Some(char_row) = sqlx::query(
        "SELECT c.user_id, u.tokens FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(error("Character not found."));
    };
    let tokens = i64::from(char_row.try_get::<i32, _>("tokens")?);
    if tokens < price {
        return Ok(json!({"status":2,"result":"Not enough tokens!"}));
    }
    let user_id: u64 = char_row.try_get("user_id")?;
    sqlx::query("UPDATE users SET tokens=tokens-?,updated_at=NOW() WHERE id=?")
        .bind(price)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    for entry in &reward_items {
        grant_deal_or_package_item(&mut tx, id, entry).await?;
    }
    tx.commit().await?;

    Ok(json!({
        "status": 1,
        "error": 0,
        "rewards": reward_items
    }))
}
