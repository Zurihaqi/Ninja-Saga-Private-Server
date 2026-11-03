//! Rank progression. Stage identifiers and response types follow the legacy AIR client.
use crate::account::{guard, integer, string};
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};

type Result = std::result::Result<Json, sqlx::Error>;
struct Exam {
    column: &'static str,
    claimed: &'static str,
    level: i32,
    prerequisite: i32,
    rank: i32,
    first: i64,
    stages: usize,
}
impl Exam {
    fn from_service(service: &str) -> Self {
        match service {
            "ChuninExam" => Self {
                column: "chunin_exam_progress",
                claimed: "chunin_claimed",
                level: 20,
                prerequisite: 1,
                rank: 3,
                first: 1,
                stages: 5,
            },
            "JouninExam" => Self {
                column: "jounin_exam_progress",
                claimed: "jounin_claimed",
                level: 40,
                prerequisite: 3,
                rank: 5,
                first: 6,
                stages: 5,
            },
            "SpecialJouninExam" => Self {
                column: "special_jounin_exam_progress",
                claimed: "special_jounin_claimed",
                level: 60,
                prerequisite: 5,
                rank: 7,
                first: 11,
                stages: 13,
            },
            _ => Self {
                column: "ninja_tutor_exam_progress",
                claimed: "ninja_tutor_claimed",
                level: 80,
                prerequisite: 7,
                rank: 9,
                first: 24,
                stages: 12,
            },
        }
    }
    fn progress(&self, value: &str) -> Vec<String> {
        let mut progress: Vec<_> = value
            .split(',')
            .filter(|s| matches!(*s, "0" | "1" | "2"))
            .map(str::to_owned)
            .take(self.stages)
            .collect();
        if progress.is_empty() {
            progress.push("1".into());
        }
        progress.resize(self.stages, "0".into());
        progress
    }
    fn data(&self, progress: &[String], claimed: bool) -> Json {
        json!(
            progress
                .iter()
                .enumerate()
                .map(|(i, p)| if self.rank == 9 {
                    json!({"id":24+i,"status":p.parse::<u8>().unwrap_or(0),"claimed":i==0 && claimed})
                } else if self.rank == 7 {
                    json!({"status":p.parse::<u8>().unwrap_or(0),"claimed":i==0 && claimed})
                } else {
                    json!({"status":p,"claimed":if i==0 && claimed {"1"} else {"0"}})
                })
                .collect::<Vec<_>>()
        )
    }
}
fn denied(message: &str) -> Json {
    json!({"status":2,"result":message})
}

pub async fn dispatch(pool: &MySqlPool, service: &str, method: &str, args: &[Json]) -> Result {
    let exam = Exam::from_service(service);
    let (id, key) =
        if (method == "extraTime" || exam.rank == 9) && args.first().is_some_and(Json::is_number) {
            (integer(args, 0), string(args, 1))
        } else {
            (integer(args, 1), string(args, 0))
        };
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let mut tx = pool.begin().await?;
    let row=sqlx::query("SELECT c.*,u.tokens FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE").bind(id).fetch_one(&mut *tx).await?;
    let mut progress = exam.progress(&row.try_get::<String, _>(exam.column)?);
    let claimed = row.try_get::<bool, _>(exam.claimed)?;
    if method == "getData" {
        return Ok(json!({"status":1,"error":0,"data":exam.data(&progress,claimed)}));
    }
    if row.try_get::<i32, _>("level")? < exam.level
        || row.try_get::<i32, _>("rank")? < exam.prerequisite
    {
        return Ok(denied(&format!(
            "You must reach level {} and the previous rank first!",
            exam.level
        )));
    }
    if method == "extraTime" {
        if row.try_get::<i32, _>("tokens")? < 30 {
            return Ok(denied("Not enough tokens!"));
        }
        sqlx::query("UPDATE users SET tokens=tokens-30 WHERE id=?")
            .bind(row.try_get::<u64, _>("user_id")?)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Ok(json!({"status":1,"result":"ok"}));
    }
    if matches!(method, "startStage" | "finishStage") {
        let index = integer(args, 2) - exam.first;
        if index < 0 || index as usize >= exam.stages {
            return Ok(denied("Invalid stage"));
        }
        let index = index as usize;
        if progress[index] == "0" {
            return Ok(denied("Stage is locked!"));
        }
        let hash = authentication::random_hex()
            .map_err(|_| sqlx::Error::Protocol("Entropy unavailable".into()))?;
        if method == "startStage" {
            let stage = if exam.rank == 9 {
                integer(args, 2)
            } else if exam.rank == 7 {
                integer(args, 2) - 5
            } else {
                index as i64 + 1
            };
            return Ok(
                json!({"status":1,"error":0,"hash":hash,"result":format!("start_stage_{stage}"),"data":exam.data(&progress,claimed)}),
            );
        }
        progress[index] = "2".into();
        if index + 1 < exam.stages && progress[index + 1] == "0" {
            progress[index + 1] = "1".into();
        }
        sqlx::query(&format!(
            "UPDATE characters SET {}=?,updated_at=NOW() WHERE id=?",
            exam.column
        ))
        .bind(progress.join(","))
        .bind(id)
        .execute(&mut *tx)
        .await?;
        // Promotion and rewards happen together in the explicit promotion call.
        // The PHP auto-promotion skipped the subsequent reward claim.
        tx.commit().await?;
        return Ok(
            json!({"status":1,"error":0,"hash":hash,"result":"Completed","data":exam.data(&progress,claimed)}),
        );
    }
    let promotion = match exam.rank {
        3 => "promoteToChunin",
        5 => "promoteToJounin",
        7 => "promoteToSpecialJounin",
        _ => "promoteToNinjaTutor",
    };
    if method != promotion {
        return Ok(denied("Unknown exam operation"));
    }
    if !progress.iter().all(|p| p == "2") {
        return Ok(denied("You must complete all exam stages first!"));
    }
    if claimed || row.try_get::<i32, _>("rank")? >= exam.rank {
        return Ok(json!({"status":1,"error":0,"result":"Promoted","rewards":[]}));
    }
    let gender = row.try_get::<i8, _>("gender")?;
    let mut rewards = Vec::<String>::new();
    let mut items = Vec::<(String, i32, &str)>::new();
    let mut skills = if exam.rank == 9 {
        vec![]
    } else {
        vec![match exam.rank {
            3 => "skill_109".to_owned(),
            5 => "skill_176".to_owned(),
            _ => "skill_345".to_owned(),
        }]
    };
    let class = string(args, 2);
    if exam.rank == 7 && !class.is_empty() {
        // A class choice must be one of the actual class skills, never arbitrary input.
        let allowed = crate::academy::valid_class(&class);
        if !allowed {
            return Ok(denied("Invalid class skill"));
        }
        skills.push(class.clone());
    }
    if exam.rank == 3 || exam.rank == 7 {
        let base = if exam.rank == 3 { "set_150" } else { "set_588" };
        let candidate = format!("{base}_{gender}");
        let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM items WHERE item_id=?")
            .bind(&candidate)
            .fetch_one(&mut *tx)
            .await?;
        items.push((if exists > 0 { candidate } else { base.into() }, 1, "set"));
    }
    if exam.rank == 3 {
        items.push(("wpn_794".into(), 1, "weapon"));
    }
    if exam.rank == 5 {
        for n in 1..=5 {
            items.push((format!("material_{n:02}"), 5, "material"));
        }
    }
    if exam.rank == 9 {
        items.extend([
            (String::from("wpn_988"), 1, "weapon"),
            (format!("set_942_{gender}"), 1, "set"),
            (String::from("back_430"), 1, "back"),
        ]);
    }
    for (item, qty, category) in items {
        let existing = sqlx::query(
            "SELECT id FROM character_items WHERE character_id=? AND item_id=? LIMIT 1",
        )
        .bind(id)
        .bind(&item)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(existing) = existing {
            sqlx::query(
                "UPDATE character_items SET quantity=quantity+?,updated_at=NOW() WHERE id=?",
            )
            .bind(qty)
            .bind(existing.try_get::<u64, _>("id")?)
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query("INSERT INTO character_items (character_id,item_id,category,quantity,created_at,updated_at) VALUES (?,?,?,?,NOW(),NOW())").bind(id).bind(&item).bind(category).bind(qty).execute(&mut *tx).await?;
        }
        rewards.push(if qty == 1 {
            item
        } else {
            format!("{item}:{qty}")
        });
    }
    for skill in skills {
        sqlx::query("INSERT INTO character_skills (character_id,skill_id,created_at,updated_at) SELECT ?,?,NOW(),NOW() WHERE NOT EXISTS (SELECT 1 FROM character_skills WHERE character_id=? AND skill_id=?)").bind(id).bind(&skill).bind(id).bind(&skill).execute(&mut *tx).await?;
        rewards.push(skill);
    }
    if exam.rank == 3 || exam.rank == 5 {
        sqlx::query(
            "UPDATE users SET tokens=tokens+200,updated_at=NOW() WHERE id=? AND tokens<=2147483447",
        )
        .bind(row.try_get::<u64, _>("user_id")?)
        .execute(&mut *tx)
        .await?;
        rewards.push("tokens_200".into());
    }
    sqlx::query(&format!("UPDATE characters SET `rank`=?,{}=1,`class`=COALESCE(?,`class`),updated_at=NOW() WHERE id=?",exam.claimed)).bind(exam.rank).bind(if exam.rank==7 && !class.is_empty(){Some(class)}else{None}).bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(json!({"status":1,"error":0,"result":"Promoted","rewards":rewards}))
}
