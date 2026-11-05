use crate::account::{guard, integer, string};
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};
use std::collections::BTreeMap;
type Result = std::result::Result<Json, sqlx::Error>;
fn denied(message: &str) -> Json {
    json!({"status":2,"result":message})
}
fn parse(value: &str) -> BTreeMap<String, i32> {
    value
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| {
            let (id, level) = s.split_once(':').unwrap_or((s, "1"));
            (id.to_owned(), level.parse().unwrap_or(1))
        })
        .collect()
}
pub async fn dispatch(pool: &MySqlPool, method: &str, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(e) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(e);
    }
    let mut tx = pool.begin().await?;
    let char = sqlx::query("SELECT * FROM characters WHERE id=? FOR UPDATE")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    let mut owned = parse(
        &char
            .try_get::<Option<String>, _>("senjutsu_skills")?
            .unwrap_or_default(),
    );
    let result = match method {
        "buyPackageSS" => {
            let (amount, cost) = match integer(args, 2) {
                0 => (10, 20),
                1 => (55, 100),
                2 => (120, 200),
                3 => (250, 400),
                _ => return Ok(denied("Invalid Sage Scroll package.")),
            };
            let user_id = char.try_get::<u64, _>("user_id")?;
            let tokens: i32 = sqlx::query_scalar("SELECT tokens FROM users WHERE id=? FOR UPDATE")
                .bind(user_id)
                .fetch_one(&mut *tx)
                .await?;
            if tokens < cost {
                return Ok(denied("Not enough tokens."));
            }
            let Some(ss) = char.try_get::<i32, _>("ss")?.checked_add(amount) else {
                return Ok(denied("SS limit reached."));
            };
            sqlx::query("UPDATE users SET tokens=tokens-?,updated_at=NOW() WHERE id=?")
                .bind(cost)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE characters SET ss=?,updated_at=NOW() WHERE id=?")
                .bind(ss)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            json!({"status":1,"ss":ss,"result":"Sage Scrolls purchased."})
        }
        "getSenjutsuSkills" => {
            let data:Vec<_>=owned.iter().map(|(id,level)|{
                let entry=crate::economy::catalog_entry("senjutsu",&format!("{id}:1"));
                json!({"id":id,"level":level,"type":entry.as_ref().and_then(|v|v["type"].as_str()).unwrap_or("other")})
            }).collect();
            return Ok(
                json!({"status":1,"data":data,"skills":data,"ss":char.try_get::<i32,_>("ss")?}),
            );
        }
        "discoverSenjutsu" => {
            let kind = string(args, 2).to_lowercase();
            if !matches!(kind.as_str(), "toad" | "snake") {
                return Ok(denied("Invalid senjutsu type"));
            }
            if char.try_get::<i32, _>("level")? < 80 {
                return Ok(denied("Level too low"));
            }
            if char
                .try_get::<Option<String>, _>("senjutsu_type")?
                .is_some_and(|v| !v.is_empty())
            {
                return Ok(denied("You already learned a Sage Mode"));
            }
            if char.try_get::<i32, _>("gold")? < 2_000_000 {
                return Ok(denied("Not enough gold"));
            }
            sqlx::query("UPDATE characters SET senjutsu_type=?,gold=gold-2000000,updated_at=NOW() WHERE id=?").bind(&kind).bind(id).execute(&mut *tx).await?;
            json!({"status":1,"result":"Senjutsu discovered!","type":kind})
        }
        "upgradeSkill" => {
            let skill = string(args, 2);
            let Some(entry) = crate::economy::catalog_entry("senjutsu", &format!("{skill}:1"))
            else {
                return Ok(denied("Senjutsu skill not found"));
            };
            let kind = char
                .try_get::<Option<String>, _>("senjutsu_type")?
                .unwrap_or_default();
            if kind.is_empty() || (entry["type"] != "other" && entry["type"] != kind) {
                return Ok(denied("Discover this Sage Mode first!"));
            }
            let current = *owned.get(&skill).unwrap_or(&0);
            if current >= 10 {
                return Ok(denied("Already at maximum level!"));
            }
            let ss = char.try_get::<i32, _>("ss")?;
            let max = integer(args, 3) == 1 || args.get(3) == Some(&Json::Bool(true));
            let mut spent = 0;
            let mut level = current;
            for next in current + 1..=10 {
                let cost = [5, 10, 25, 50, 100, 200, 300, 450, 600, 800][(next - 1) as usize];
                if spent + cost > ss {
                    break;
                }
                spent += cost;
                level = next;
                if !max {
                    break;
                }
            }
            if level == current {
                return Ok(denied("Not enough SS!"));
            }
            owned.insert(skill, level);
            let owned = owned
                .iter()
                .map(|(id, lv)| format!("{id}:{lv}"))
                .collect::<Vec<_>>()
                .join(",");
            sqlx::query(
                "UPDATE characters SET ss=ss-?,senjutsu_skills=?,updated_at=NOW() WHERE id=?",
            )
            .bind(spent)
            .bind(owned)
            .bind(id)
            .execute(&mut *tx)
            .await?;
            json!({"status":1,"spent_ss":spent,"result":"Senjutsu upgraded successfully!"})
        }
        "equipSkill" => {
            let skills = if let Some(list) = args.get(2).and_then(Json::as_array) {
                list.iter()
                    .filter_map(Json::as_str)
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            } else {
                string(args, 2).split(',').map(str::to_owned).collect()
            };
            let mut equipped = vec![];
            for skill in skills {
                let skill = skill.trim();
                if owned.contains_key(skill)
                    && !equipped.contains(&skill.to_owned())
                    && equipped.len() < 8
                {
                    equipped.push(skill.to_owned());
                }
            }
            let equipped = equipped.join(",");
            sqlx::query(
                "UPDATE characters SET senjutsu_equipped_skills=?,updated_at=NOW() WHERE id=?",
            )
            .bind(&equipped)
            .bind(id)
            .execute(&mut *tx)
            .await?;
            json!({"status":1,"skills":equipped})
        }
        _ => return Ok(denied("Unknown senjutsu operation")),
    };
    tx.commit().await?;
    Ok(result)
}
