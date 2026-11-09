//! Element/Sage reset contracts used by ResetSkill and ResetSenjutsu.
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
pub async fn dispatch(pool: &MySqlPool, method: &str, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(e) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(e);
    }
    let mut tx = pool.begin().await?;
    let c=sqlx::query("SELECT c.*,u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE").bind(id).fetch_one(&mut *tx).await?;
    let required = if c.try_get::<i8, _>("account_type")? > 0 {
        1
    } else {
        5
    };
    if method == "resetElements" {
        let Some(selected) = args.get(2).and_then(Json::as_array) else {
            return Ok(denied("Select an element first."));
        };
        let selected: Vec<i64> = selected
            .iter()
            .filter_map(|v| {
                v.as_i64()
                    .or_else(|| v.as_str().and_then(|v| v.parse().ok()))
            })
            .collect();
        let owned = [
            c.try_get::<i32, _>("element_1")?,
            c.try_get::<i32, _>("element_2")?,
            c.try_get::<i32, _>("element_3")?,
        ];
        if selected.is_empty()
            || selected.len() > 3
            || selected
                .iter()
                .any(|v| !(1..=5).contains(v) || !owned.contains(&(*v as i32)))
        {
            return Ok(denied("Select an element you have learned."));
        }
        let mut unique = selected.clone();
        unique.sort_unstable();
        unique.dedup();
        if unique.len() != selected.len() {
            return Ok(denied("Duplicate element selection."));
        }
        if !inventory::take(&mut tx, id, "material_1001", required).await? {
            return Ok(denied("Not enough Ninja Seal Gan."));
        }
        for element in &selected {
            sqlx::query("DELETE cs FROM character_skills cs JOIN skills s ON s.skill_id=cs.skill_id WHERE cs.character_id=? AND s.element=?").bind(id).bind(element).execute(&mut *tx).await?;
        }
        let remaining: Vec<i32> = owned
            .into_iter()
            .filter(|v| *v > 0 && !selected.contains(&i64::from(*v)))
            .collect();
        sqlx::query("UPDATE characters SET element_1=?,element_2=?,element_3=?,equipment_skills='',updated_at=NOW() WHERE id=?").bind(remaining.first().copied().unwrap_or(0)).bind(remaining.get(1).copied().unwrap_or(0)).bind(remaining.get(2).copied().unwrap_or(0)).bind(id).execute(&mut *tx).await?;
        sqlx::query("UPDATE character_skill_sets SET skills='' WHERE character_id=?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
    } else if method == "resetSenjutsu" {
        let kind = c
            .try_get::<Option<String>, _>("senjutsu_type")?
            .unwrap_or_default();
        if kind.is_empty() {
            return Ok(denied("You have not discovered Sage Mode."));
        }
        if !args
            .get(2)
            .and_then(Json::as_array)
            .is_some_and(|a| a.len() == 1 && a[0] == kind)
        {
            return Ok(denied("Select your current Sage Mode."));
        }
        if !inventory::take(&mut tx, id, "essential_11", required).await? {
            return Ok(denied("Not enough Sage reset seals."));
        }
        let skills = c
            .try_get::<Option<String>, _>("senjutsu_skills")?
            .unwrap_or_default();
        let costs = [5, 10, 25, 50, 100, 200, 300, 450, 600, 800];
        let refund: i32 = skills
            .split(',')
            .filter_map(|v| v.split_once(':'))
            .filter_map(|(_, v)| v.parse::<usize>().ok())
            .map(|level| costs.iter().take(level.min(10)).sum::<i32>())
            .sum();
        let Some(ss) = c.try_get::<i32, _>("ss")?.checked_add(refund) else {
            return Ok(denied("SS limit reached."));
        };
        sqlx::query("UPDATE characters SET senjutsu_type=NULL,senjutsu_skills='',senjutsu_equipped_skills='',ss=?,updated_at=NOW() WHERE id=?").bind(ss).bind(id).execute(&mut *tx).await?;
    } else {
        return Ok(denied("Invalid reset action."));
    }
    tx.commit().await?;
    Ok(json!({"status":1}))
}
