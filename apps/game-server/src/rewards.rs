//! Shared reward string parser and inventory/currency/XP grant helpers.
//! Source: `app/Services/Amf/RewardGrantService.php` and `app/Models/Character.php`.
use persistence::{Row, sqlx};

type Tx<'a> = sqlx::Transaction<'a, sqlx::MySql>;

pub(crate) fn rank_cap(rank: i32) -> i32 {
    match rank {
        1 => 20,
        3 => 40,
        5 => 60,
        7 => 80,
        _ => 85,
    }
}

pub(crate) fn parse_amount(reward_id: &str, prefix: &str) -> i64 {
    let raw = reward_id.strip_prefix(prefix).unwrap_or("");
    let digits: String = raw.chars().filter(char::is_ascii_digit).collect();
    digits.parse::<i64>().unwrap_or(0)
}

pub(crate) async fn resolve_category(
    tx: &mut Tx<'_>,
    item_id: &str,
) -> Result<String, sqlx::Error> {
    let prefix = item_id.split('_').next().unwrap_or("");
    let slot_category = match prefix {
        "wpn" => Some("weapon"),
        "back" => Some("back"),
        "accessory" => Some("accessory"),
        "set" => Some("set"),
        "hair" => Some("hair"),
        "material" => Some("material"),
        "essential" => Some("essential"),
        _ => None,
    };
    if let Some(category) = slot_category {
        return Ok(category.into());
    }
    let db_category: Option<String> =
        sqlx::query_scalar("SELECT category FROM items WHERE item_id=? LIMIT 1")
            .bind(item_id)
            .fetch_optional(&mut **tx)
            .await?;
    Ok(db_category
        .filter(|c| !c.is_empty())
        .unwrap_or_else(|| "item".into()))
}

pub(crate) async fn give_item(
    tx: &mut Tx<'_>,
    char_id: i64,
    item_id: &str,
    qty: i64,
) -> Result<(), sqlx::Error> {
    if qty <= 0 {
        return Ok(());
    }
    let row = sqlx::query(
        "SELECT id,quantity FROM character_items WHERE character_id=? AND item_id=? LIMIT 1 FOR UPDATE",
    )
    .bind(char_id)
    .bind(item_id)
    .fetch_optional(&mut **tx)
    .await?;
    if let Some(row) = row {
        let quantity = i64::from(row.try_get::<i32, _>("quantity")?)
            .checked_add(qty)
            .filter(|n| *n <= i64::from(i32::MAX) && *n > 0)
            .ok_or_else(|| sqlx::Error::Protocol("Inventory overflow".into()))?;
        sqlx::query("UPDATE character_items SET quantity=?,updated_at=NOW() WHERE id=?")
            .bind(quantity)
            .bind(row.try_get::<u64, _>("id")?)
            .execute(&mut **tx)
            .await?;
    } else {
        let category = resolve_category(tx, item_id).await?;
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

pub(crate) async fn add_xp(
    tx: &mut Tx<'_>,
    char_id: i64,
    amount: i64,
) -> Result<(i32, i64, bool), sqlx::Error> {
    let row = sqlx::query("SELECT xp,level,`rank` FROM characters WHERE id=? FOR UPDATE")
        .bind(char_id)
        .fetch_one(&mut **tx)
        .await?;
    let old_level: i32 = row.try_get("level")?;
    let rank: i32 = row.try_get("rank")?;
    let mut level = old_level;
    let cap = rank_cap(rank);
    let mut xp = row
        .try_get::<i64, _>("xp")?
        .checked_add(amount)
        .ok_or_else(|| sqlx::Error::Protocol("XP overflow".into()))?;
    while level < cap {
        let required: Option<i64> =
            sqlx::query_scalar("SELECT character_xp FROM x_p_s WHERE level=?")
                .bind(level)
                .fetch_optional(&mut **tx)
                .await?;
        if xp >= required.filter(|v| *v > 0).unwrap_or(999999999) {
            level += 1;
        } else {
            break;
        }
    }
    if level >= cap
        && let Some(required) =
            sqlx::query_scalar::<_, i64>("SELECT character_xp FROM x_p_s WHERE level=?")
                .bind(level)
                .fetch_optional(&mut **tx)
                .await?
        && required > 0
    {
        xp = xp.min(required);
    }
    sqlx::query("UPDATE characters SET xp=?,level=?,updated_at=NOW() WHERE id=?")
        .bind(xp)
        .bind(level)
        .bind(char_id)
        .execute(&mut **tx)
        .await?;
    Ok((level, xp, level > old_level))
}

pub(crate) async fn add_gold(
    tx: &mut Tx<'_>,
    char_id: i64,
    amount: i64,
) -> Result<(), sqlx::Error> {
    if amount <= 0 {
        return Ok(());
    }
    sqlx::query(
        "UPDATE characters SET gold = LEAST(2147483647, gold + ?), updated_at=NOW() WHERE id=?",
    )
    .bind(amount)
    .bind(char_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Grants a reward string (`gold_X`, `tokens_X`, `tp_X`, `ss_X`, `xp_percent_X`, `xp_X%`,
/// `xp_X`, `pet_X`, `skill_X`, or `item_id[:qty]` with `%s` gender substitution) and returns
/// `Ok(true)` if the character leveled up.
pub(crate) async fn grant(
    tx: &mut Tx<'_>,
    char_id: i64,
    reward_str: &str,
) -> Result<bool, sqlx::Error> {
    let reward_str = reward_str.trim();
    if reward_str.is_empty() {
        return Ok(false);
    }
    let (raw_id, qty_str) = reward_str
        .split_once(':')
        .map_or((reward_str, None), |(id, q)| (id, Some(q)));
    let quantity = qty_str
        .and_then(|s| s.parse::<i64>().ok())
        .filter(|q| *q > 0)
        .unwrap_or(1);

    let char_row = sqlx::query(
        "SELECT user_id,gender,level,`rank`,gold,tp,ss FROM characters WHERE id=? FOR UPDATE",
    )
    .bind(char_id)
    .fetch_one(&mut **tx)
    .await?;
    let gender: i8 = char_row.try_get("gender")?;
    let suffix = if gender == 0 { "0" } else { "1" };
    let reward_id = if raw_id.contains("%s") {
        raw_id.replace("%s", suffix)
    } else {
        raw_id.to_owned()
    };

    if reward_id.starts_with("gold_") {
        let amount = parse_amount(&reward_id, "gold_") * quantity;
        let new_gold = i64::from(char_row.try_get::<i32, _>("gold")?)
            .saturating_add(amount)
            .min(i64::from(i32::MAX));
        sqlx::query("UPDATE characters SET gold=?,updated_at=NOW() WHERE id=?")
            .bind(new_gold)
            .bind(char_id)
            .execute(&mut **tx)
            .await?;
        return Ok(false);
    }
    if reward_id.starts_with("tokens_") {
        let amount = parse_amount(&reward_id, "tokens_") * quantity;
        let user_id: u64 = char_row.try_get("user_id")?;
        sqlx::query("UPDATE users SET tokens=tokens+?,updated_at=NOW() WHERE id=?")
            .bind(amount)
            .bind(user_id)
            .execute(&mut **tx)
            .await?;
        return Ok(false);
    }
    if reward_id.starts_with("tp_") {
        let amount = parse_amount(&reward_id, "tp_") * quantity;
        let new_tp = i64::from(char_row.try_get::<i32, _>("tp")?)
            .saturating_add(amount)
            .min(i64::from(i32::MAX));
        sqlx::query("UPDATE characters SET tp=?,updated_at=NOW() WHERE id=?")
            .bind(new_tp)
            .bind(char_id)
            .execute(&mut **tx)
            .await?;
        return Ok(false);
    }
    if reward_id.starts_with("ss_") {
        let amount = parse_amount(&reward_id, "ss_") * quantity;
        let new_ss = i64::from(char_row.try_get::<i32, _>("ss")?)
            .saturating_add(amount)
            .min(i64::from(i32::MAX));
        sqlx::query("UPDATE characters SET ss=?,updated_at=NOW() WHERE id=?")
            .bind(new_ss)
            .bind(char_id)
            .execute(&mut **tx)
            .await?;
        return Ok(false);
    }
    if reward_id.starts_with("xp_percent_")
        || (reward_id.starts_with("xp_") && reward_id.contains('%'))
    {
        let prefix = if reward_id.starts_with("xp_percent_") {
            "xp_percent_"
        } else {
            "xp_"
        };
        let percent = parse_amount(&reward_id, prefix);
        let level: i32 = char_row.try_get("level")?;
        let rank: i32 = char_row.try_get("rank")?;
        if level < rank_cap(rank) && percent > 0 {
            let xp_req: i64 = sqlx::query_scalar("SELECT character_xp FROM x_p_s WHERE level=?")
                .bind(level)
                .fetch_optional(&mut **tx)
                .await?
                .filter(|v| *v > 0)
                .unwrap_or(1000);
            let amount = (xp_req * percent / 100) * quantity;
            let (_, _, level_up) = add_xp(tx, char_id, amount).await?;
            return Ok(level_up);
        }
        return Ok(false);
    }
    if reward_id.starts_with("xp_") {
        let amount = parse_amount(&reward_id, "xp_");
        let level: i32 = char_row.try_get("level")?;
        let rank: i32 = char_row.try_get("rank")?;
        if level >= rank_cap(rank) {
            return Ok(false);
        }
        let (_, _, level_up) = add_xp(tx, char_id, amount * quantity).await?;
        return Ok(level_up);
    }
    if reward_id.starts_with("pet_") {
        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM character_pets WHERE character_id=? AND pet_id=?",
        )
        .bind(char_id)
        .bind(&reward_id)
        .fetch_one(&mut **tx)
        .await?;
        if exists == 0 {
            sqlx::query(
                "INSERT INTO character_pets (character_id,pet_id,name,level,xp,created_at,updated_at) VALUES (?,?,'Pet',1,0,NOW(),NOW())",
            )
            .bind(char_id)
            .bind(&reward_id)
            .execute(&mut **tx)
            .await?;
        }
        return Ok(false);
    }
    if reward_id.starts_with("skill_") {
        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM character_skills WHERE character_id=? AND skill_id=?",
        )
        .bind(char_id)
        .bind(&reward_id)
        .fetch_one(&mut **tx)
        .await?;
        if exists == 0 {
            sqlx::query(
                "INSERT INTO character_skills (character_id,skill_id,created_at,updated_at) VALUES (?,?,NOW(),NOW())",
            )
            .bind(char_id)
            .bind(&reward_id)
            .execute(&mut **tx)
            .await?;
        }
        return Ok(false);
    }
    give_item(tx, char_id, &reward_id, quantity).await?;
    Ok(false)
}
