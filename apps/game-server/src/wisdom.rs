//! Secret Scroll of Wisdom uses the exact elemental lists shipped with the client.
use crate::{
    account::{guard, integer, string},
    inventory,
};
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};
type Result = std::result::Result<Json, sqlx::Error>;
pub async fn dispatch(pool: &MySqlPool, method: &str, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(e) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(e);
    }
    let mut tx = pool.begin().await?;
    let c =
        sqlx::query("SELECT element_1,element_2,element_3 FROM characters WHERE id=? FOR UPDATE")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    if method == "getData" {
        let count:i64=sqlx::query_scalar("SELECT COALESCE(CAST(SUM(quantity) AS SIGNED),0) FROM character_items WHERE character_id=? AND item_id='essential_10'").bind(id).fetch_one(&mut *tx).await?;
        return Ok(json!({"status":1,"scrolls":count}));
    }
    if method != "claimSkill" {
        return Ok(json!({"status":2,"result":"Invalid scroll action."}));
    }
    let element = integer(args, 2);
    let elements = [
        c.try_get::<i32, _>("element_1")?,
        c.try_get::<Option<i32>, _>("element_2")?.unwrap_or(0),
        c.try_get::<Option<i32>, _>("element_3")?.unwrap_or(0),
    ];
    if !(1..=5).contains(&element) || !elements.contains(&(element as i32)) {
        return Ok(json!({"status":2,"result":"Learn this element first."}));
    }
    let catalog: Json =
        serde_json::from_str(include_str!("../../../database/catalogs/wisdom.json"))
            .expect("compiled wisdom catalog");
    let skills = catalog[["wind", "fire", "thunder", "earth", "water"][(element - 1) as usize]]
        .as_array()
        .expect("elemental skill list");
    let mut added = 0;
    for skill in skills {
        let skill = skill.as_str().expect("skill id");
        let owned: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM character_skills WHERE character_id=? AND skill_id=?",
        )
        .bind(id)
        .bind(skill)
        .fetch_one(&mut *tx)
        .await?;
        if owned == 0 {
            sqlx::query("INSERT INTO character_skills(character_id,skill_id,created_at,updated_at) VALUES(?,?,NOW(),NOW())").bind(id).bind(skill).execute(&mut *tx).await?;
            added += 1;
        }
    }
    if added == 0 {
        return Ok(json!({"status":2,"result":"You already know every skill in this scroll."}));
    }
    if !inventory::take(&mut tx, id, "essential_10", 1).await? {
        return Ok(json!({"status":2,"result":"You need a Secret Scroll of Wisdom."}));
    }
    let all: Vec<String> = sqlx::query_scalar(
        "SELECT skill_id FROM character_skills WHERE character_id=? ORDER BY id",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(json!({"status":1,"skills":all.join(",")}))
}
