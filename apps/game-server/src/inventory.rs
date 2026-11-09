//! Inventory mutations run under the owning character's transaction/row lock.
use persistence::{Row, sqlx};
type Tx<'a> = sqlx::Transaction<'a, sqlx::MySql>;
pub async fn take(tx: &mut Tx<'_>, id: i64, item: &str, qty: i64) -> Result<bool, sqlx::Error> {
    let row=sqlx::query("SELECT id,quantity FROM character_items WHERE character_id=? AND item_id=? LIMIT 1 FOR UPDATE").bind(id).bind(item).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(false);
    };
    let available = i64::from(row.try_get::<i32, _>("quantity")?);
    if qty <= 0 || available < qty {
        return Ok(false);
    }
    if available == qty {
        sqlx::query("DELETE FROM character_items WHERE id=?")
            .bind(row.try_get::<u64, _>("id")?)
            .execute(&mut **tx)
            .await?;
    } else {
        sqlx::query("UPDATE character_items SET quantity=quantity-?,updated_at=NOW() WHERE id=?")
            .bind(qty)
            .bind(row.try_get::<u64, _>("id")?)
            .execute(&mut **tx)
            .await?;
    }
    Ok(true)
}
pub async fn give(tx: &mut Tx<'_>, id: i64, item: &str, qty: i64) -> Result<(), sqlx::Error> {
    let row=sqlx::query("SELECT id,quantity FROM character_items WHERE character_id=? AND item_id=? LIMIT 1 FOR UPDATE").bind(id).bind(item).fetch_optional(&mut **tx).await?;
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
        let prefix = item.split('_').next().unwrap_or("item");
        let category = match prefix {
            "wpn" => "weapon",
            v => v,
        };
        sqlx::query("INSERT INTO character_items (character_id,item_id,category,quantity,created_at,updated_at) VALUES (?,?,?,?,NOW(),NOW())").bind(id).bind(item).bind(category).bind(qty).execute(&mut **tx).await?;
    }
    Ok(())
}
