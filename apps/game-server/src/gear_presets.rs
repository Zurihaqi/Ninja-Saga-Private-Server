//! Gear panel contracts from Panels.UI_Gear in the pinned client.
use crate::account::{guard, integer, string};
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};
type Result = std::result::Result<Json, sqlx::Error>;
fn denied(message: &str) -> Json {
    json!({"status":2,"result":message})
}

pub async fn dispatch(pool: &MySqlPool, method: &str, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(e) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(e);
    }
    let mut tx = pool.begin().await?;
    let c = sqlx::query("SELECT c.*,u.tokens FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE").bind(id).fetch_one(&mut *tx).await?;
    match method {
        "getGearPresets" => {}
        "unlockGearPresetSlot" => {
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM character_gear_presets WHERE character_id=?",
            )
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
            if count >= 4 {
                return Ok(denied("Maximum 4 gear presets allowed."));
            }
            let cost = if count == 0 { 0 } else { 500 };
            let tokens = c.try_get::<i32, _>("tokens")?;
            if tokens < cost {
                return Ok(denied("Not enough tokens."));
            }
            sqlx::query("UPDATE users SET tokens=tokens-?,updated_at=NOW() WHERE id=?")
                .bind(cost)
                .bind(c.try_get::<u64, _>("user_id")?)
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO character_gear_presets(character_id,name,weapon,back_item,clothing,accessory,hair,hair_color) VALUES(?,?,?,?,?,?,?,?)").bind(id).bind(format!("Preset {}",count+1))
                .bind(c.try_get::<Option<String>,_>("equipment_weapon")?.unwrap_or_default()).bind(c.try_get::<Option<String>,_>("equipment_back")?.unwrap_or_default()).bind(c.try_get::<Option<String>,_>("equipment_clothing")?.unwrap_or_default()).bind(c.try_get::<Option<String>,_>("equipment_accessory")?.unwrap_or_default()).bind(c.try_get::<Option<String>,_>("hair_style")?.unwrap_or_default()).bind(c.try_get::<Option<String>,_>("hair_color")?.unwrap_or_default()).execute(&mut *tx).await?;
            tx.commit().await?;
            return Ok(json!({"status":1,"tokens":tokens-cost}));
        }
        "renameGearPreset" | "updateGearPreset" => {
            let preset = integer(args, 2);
            let owned: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM character_gear_presets WHERE character_id=? AND id=?",
            )
            .bind(id)
            .bind(preset)
            .fetch_one(&mut *tx)
            .await?;
            if owned == 0 {
                return Ok(denied("Preset not found."));
            }
            if method == "renameGearPreset" {
                let name = string(args, 3).trim().to_owned();
                if name.is_empty()
                    || name.chars().count() > 32
                    || name.chars().any(char::is_control)
                {
                    return Ok(denied("Use a name between 1 and 32 characters."));
                }
                sqlx::query(
                    "UPDATE character_gear_presets SET name=? WHERE id=? AND character_id=?",
                )
                .bind(name)
                .bind(preset)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            } else {
                // The client sends weapon, clothing, hair, back, accessory, color.
                for (index, prefix) in [
                    (3, "wpn_"),
                    (4, "set_"),
                    (5, "hair_"),
                    (6, "back_"),
                    (7, "accessory_"),
                ] {
                    let item = string(args, index);
                    if item.is_empty() {
                        continue;
                    }
                    // Existing characters received a starter hairstyle directly,
                    // without a duplicate inventory row in the original schema.
                    if index == 5
                        && c.try_get::<Option<String>, _>("hair_style")?.as_deref()
                            == Some(item.as_str())
                    {
                        continue;
                    }
                    if !item.starts_with(prefix) {
                        return Ok(denied("Invalid gear category."));
                    }
                    let owned:i64=sqlx::query_scalar("SELECT COUNT(*) FROM character_items WHERE character_id=? AND item_id=? AND quantity>0").bind(id).bind(&item).fetch_one(&mut *tx).await?;
                    if owned == 0 {
                        return Ok(denied("Gear is not owned."));
                    }
                }
                let color = string(args, 8);
                if color.len() > 255 {
                    return Ok(denied("Invalid hair color."));
                }
                sqlx::query("UPDATE character_gear_presets SET weapon=?,clothing=?,hair=?,back_item=?,accessory=?,hair_color=? WHERE id=? AND character_id=?").bind(string(args,3)).bind(string(args,4)).bind(string(args,5)).bind(string(args,6)).bind(string(args,7)).bind(color).bind(preset).bind(id).execute(&mut *tx).await?;
            }
            tx.commit().await?;
            return Ok(json!({"status":1}));
        }
        _ => return Ok(denied("Invalid gear preset action.")),
    }
    let rows = sqlx::query("SELECT * FROM character_gear_presets WHERE character_id=? ORDER BY id")
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
    let presets:Vec<Json>=rows.iter().map(|r|json!({"id":r.get::<u64,_>("id"),"name":r.get::<String,_>("name"),"weapon":r.get::<String,_>("weapon"),"clothing":r.get::<String,_>("clothing"),"hair":r.get::<String,_>("hair"),"back_item":r.get::<String,_>("back_item"),"accessory":r.get::<String,_>("accessory"),"hair_color":r.get::<String,_>("hair_color")})).collect();
    Ok(json!({"status":1,"presets":presets}))
}
