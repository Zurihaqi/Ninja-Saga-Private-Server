use crate::account::{guard, integer, session_error, string};
use crate::rewards;
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};

type Result = std::result::Result<Json, sqlx::Error>;

pub async fn dispatch(pool: &MySqlPool, method: &str, args: &[Json]) -> Result {
    match method {
        "getMails" => get_mails(pool, args).await,
        "openMail" => open_mail(pool, args).await,
        "deleteMail" => delete_mail(pool, args).await,
        "deleteAllMails" => delete_all_mails(pool, args).await,
        "claimReward" | "claimRewards" => claim_reward(pool, args).await,
        "claimAllRewards" => claim_all_rewards(pool, args).await,
        "acceptInvitation" => accept_invitation(pool, args).await,
        "acceptFriendRequest" => accept_friend_request(pool, args).await,
        _ => Ok(session_error(&format!(
            "Invalid MailService method: {method}"
        ))),
    }
}

fn format_mail_list(rows: Vec<sqlx::mysql::MySqlRow>) -> Vec<Json> {
    rows.into_iter()
        .map(|m| {
            let id: u64 = m.try_get("id").unwrap_or(0);
            let title: String = m.try_get("title").unwrap_or_default();
            let sender: String = m
                .try_get::<Option<String>, _>("sender")
                .ok()
                .flatten()
                .unwrap_or_else(|| "System".into());
            let body: String = m
                .try_get::<Option<String>, _>("body")
                .ok()
                .flatten()
                .unwrap_or_default();
            let mail_type: String = m
                .try_get::<Option<String>, _>("type")
                .ok()
                .flatten()
                .unwrap_or_else(|| "system".into());
            let rewards_arr: Vec<Json> = m
                .try_get::<Option<Json>, _>("rewards")
                .ok()
                .flatten()
                .and_then(|j| j.as_array().cloned())
                .unwrap_or_default();
            let claimed: bool = m.try_get("claimed").unwrap_or(false);
            let viewed: bool = m.try_get("viewed").unwrap_or(false);
            let date: String = m
                .try_get::<Option<String>, _>("date_str")
                .ok()
                .flatten()
                .unwrap_or_default();
            json!({
                "id": id,
                "mail_id": id,
                "title": title,
                "sender": sender,
                "body": body,
                "type": mail_type,
                "rewards": rewards_arr,
                "claimed": claimed,
                "viewed": viewed,
                "date": date
            })
        })
        .collect()
}

async fn load_mails_for_char(
    pool: &MySqlPool,
    id: i64,
) -> std::result::Result<Vec<Json>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, title, sender, body, `type`, CAST(rewards AS CHAR) AS rewards, claimed, viewed, \
         DATE_FORMAT(created_at, '%Y-%m-%d %H:%i') AS date_str \
         FROM character_mails WHERE character_id=? ORDER BY created_at DESC, id DESC",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    Ok(format_mail_list(rows))
}

async fn get_mails(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let mails = load_mails_for_char(pool, id).await?;
    Ok(json!({
        "status": 1,
        "error": 0,
        "mails": mails
    }))
}

async fn open_mail(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let mail_id = integer(args, 2);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    sqlx::query(
        "UPDATE character_mails SET viewed=1, updated_at=NOW() WHERE character_id=? AND id=?",
    )
    .bind(id)
    .bind(mail_id)
    .execute(pool)
    .await?;
    Ok(json!({"status":1,"error":0}))
}

async fn delete_mail(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let mail_id = integer(args, 2);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    sqlx::query("DELETE FROM character_mails WHERE character_id=? AND id=?")
        .bind(id)
        .bind(mail_id)
        .execute(pool)
        .await?;
    let mails = load_mails_for_char(pool, id).await?;
    Ok(json!({
        "status": 1,
        "error": 0,
        "mails": mails
    }))
}

async fn delete_all_mails(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    sqlx::query("DELETE FROM character_mails WHERE character_id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(json!({
        "status": 1,
        "error": 0,
        "mails": []
    }))
}

async fn claim_reward(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let mail_id = integer(args, 2);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let mut tx = pool.begin().await?;
    let Some(mail) = sqlx::query(
        "SELECT id, CAST(rewards AS CHAR) AS rewards, claimed FROM character_mails WHERE character_id=? AND id=? FOR UPDATE",
    )
    .bind(id)
    .bind(mail_id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(json!({"status":2,"result":"Nothing to claim."}));
    };
    let claimed: bool = mail.try_get("claimed")?;
    let rewards_raw: Option<String> = mail.try_get("rewards").ok().flatten();
    let rewards_arr: Vec<String> = rewards_raw
        .as_deref()
        .and_then(|s| serde_json::from_str::<Json>(s).ok())
        .and_then(|j| j.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    if claimed || rewards_arr.is_empty() {
        return Ok(json!({"status":2,"result":"Nothing to claim."}));
    }

    let mut granted = Vec::new();
    let mut level_up = false;
    for r_str in &rewards_arr {
        if !r_str.is_empty() {
            if rewards::grant(&mut tx, id, r_str).await? {
                level_up = true;
            }
            granted.push(r_str.clone());
        }
    }
    sqlx::query("UPDATE character_mails SET claimed=1, viewed=1, updated_at=NOW() WHERE id=?")
        .bind(mail_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let mails = load_mails_for_char(pool, id).await?;
    Ok(json!({
        "status": 1,
        "error": 0,
        "rewards": granted,
        "level_up": level_up,
        "mails": mails,
        "result": "Reward claimed!"
    }))
}

async fn claim_all_rewards(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let mut tx = pool.begin().await?;
    let unclaimed = sqlx::query(
        "SELECT id, CAST(rewards AS CHAR) AS rewards FROM character_mails WHERE character_id=? AND claimed=0 AND rewards IS NOT NULL FOR UPDATE",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;

    let mut granted = Vec::new();
    let mut level_up = false;
    for mail in unclaimed {
        let mid: u64 = mail.try_get("id")?;
        let rewards_raw: Option<String> = mail.try_get("rewards").ok().flatten();
        let rewards_arr: Vec<String> = rewards_raw
            .as_deref()
            .and_then(|s| serde_json::from_str::<Json>(s).ok())
            .and_then(|j| j.as_array().cloned())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        for r_str in &rewards_arr {
            if !r_str.is_empty() {
                if rewards::grant(&mut tx, id, r_str).await? {
                    level_up = true;
                }
                granted.push(r_str.clone());
            }
        }
        sqlx::query("UPDATE character_mails SET claimed=1, viewed=1, updated_at=NOW() WHERE id=?")
            .bind(mid)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;

    let mails = load_mails_for_char(pool, id).await?;
    Ok(json!({
        "status": 1,
        "error": 0,
        "rewards": granted,
        "level_up": level_up,
        "mails": mails,
        "result": if granted.is_empty() { "No unclaimed rewards." } else { "All rewards claimed!" }
    }))
}

async fn accept_invitation(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let mail_id = integer(args, 2);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    sqlx::query("DELETE FROM character_mails WHERE character_id=? AND id=?")
        .bind(id)
        .bind(mail_id)
        .execute(pool)
        .await?;
    let mails = load_mails_for_char(pool, id).await?;
    Ok(json!({
        "status": 1,
        "error": 0,
        "mails": mails,
        "result": "Invitation accepted."
    }))
}

async fn accept_friend_request(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let mail_id = integer(args, 2);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    sqlx::query("DELETE FROM character_mails WHERE character_id=? AND id=?")
        .bind(id)
        .bind(mail_id)
        .execute(pool)
        .await?;
    let mails = load_mails_for_char(pool, id).await?;
    Ok(json!({
        "status": 1,
        "error": 0,
        "mails": mails,
        "result": "Friend request accepted."
    }))
}
