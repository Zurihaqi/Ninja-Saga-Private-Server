use crate::account::{guard, integer, string};
use crate::rewards;
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};
use std::collections::HashSet;

type Result = std::result::Result<Json, sqlx::Error>;

const FRIENDS_PER_PAGE: i64 = 8;
const REQUESTS_PER_PAGE: i64 = 4;
const RECOMMENDATIONS_PER_PAGE: i64 = 5;
const FREE_FRIEND_LIMIT: i64 = 50;
const PREMIUM_FRIEND_LIMIT: i64 = 100;
const FRIENDSHIP_KUNAI: &str = "material_1002";

pub async fn dispatch(pool: &MySqlPool, method: &str, args: &[Json]) -> Result {
    match method {
        "addFriend" => add_friend(pool, args).await,
        "friendRequests" => friend_requests(pool, args).await,
        "acceptFriend" => accept_friend(pool, args).await,
        "acceptAll" => accept_all(pool, args).await,
        "removeAll" => remove_all(pool, args).await,
        "friends" => friends(pool, args).await,
        "getFavorite" => get_favorite(pool, args).await,
        "search" => search(pool, args).await,
        "getRecommendations" => get_recommendations(pool, args).await,
        "setFavorite" => set_favorite(pool, args).await,
        "removeFavorite" => remove_favorite(pool, args).await,
        "removeFriend" => remove_friend(pool, args).await,
        "unfriendAll" => unfriend_all(pool, args).await,
        "recruitable" => recruitable(pool, args).await,
        "recruitFriend" => recruit_friend(pool, args).await,
        "startBerantem" => start_berantem(pool, args).await,
        "endBerantem" => end_berantem(pool, args).await,
        "getItems" => get_items(pool, args).await,
        "buyItem" => buy_item(pool, args).await,
        _ => Ok(json!({"status":0,"error":format!("FriendService.{method} not found")})),
    }
}

fn format_character_for_friend(row: &sqlx::mysql::MySqlRow, is_favorite: bool) -> Json {
    let id: u64 = row.try_get("id").unwrap_or(0);
    let name: String = row.try_get("name").unwrap_or_default();
    let level: i32 = row.try_get("level").unwrap_or(1);
    let rank: i32 = row.try_get("rank").unwrap_or(1);
    let gender: i8 = row.try_get("gender").unwrap_or(0);
    let suffix = if gender == 0 { "_0" } else { "_1" };
    let hair_raw: Option<String> = row.try_get("hair_style").ok().flatten();
    let hair = match hair_raw.as_deref() {
        Some(h) if !h.is_empty() && h.chars().all(|c| c.is_ascii_digit()) => {
            format!("hair_{:0>2}{suffix}", h)
        }
        Some(h) if !h.is_empty() => h.to_string(),
        _ => format!("hair_01{suffix}"),
    };
    let weapon = row
        .try_get::<Option<String>, _>("equipment_weapon")
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "wpn_01".into());
    let back_item = row
        .try_get::<Option<String>, _>("equipment_back")
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "back_01".into());
    let clothing = row
        .try_get::<Option<String>, _>("equipment_clothing")
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("set_01{suffix}"));
    let accessory = row
        .try_get::<Option<String>, _>("equipment_accessory")
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "accessory_01".into());
    let hair_color = row
        .try_get::<Option<String>, _>("hair_color")
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "0|0".into());
    let skin_color = row
        .try_get::<Option<String>, _>("skin_color")
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "0|0".into());
    let pet_val = match row
        .try_get::<Option<String>, _>("equipment_pet")
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
    {
        Some(p) => json!(p),
        None => json!(0),
    };
    let account_type: i8 = row.try_get("account_type").unwrap_or(0);
    json!({
        "id": id,
        "char_id": id,
        "character_name": name,
        "character_level": level,
        "character_rank": rank,
        "character_gender": gender,
        "account_type": account_type,
        "is_favorite": is_favorite,
        "sets": {
            "weapon": weapon,
            "back_item": back_item,
            "clothing": clothing,
            "hairstyle": hair,
            "accessory": accessory,
            "face": format!("face_01{suffix}"),
            "hair_color": hair_color,
            "skin_color": skin_color,
            "pet": pet_val
        }
    })
}

async fn kunai_count(pool: &MySqlPool, id: i64) -> Result {
    let qty: Option<i32> = sqlx::query_scalar(
        "SELECT quantity FROM character_items WHERE character_id=? AND item_id=?",
    )
    .bind(id)
    .bind(FRIENDSHIP_KUNAI)
    .fetch_optional(pool)
    .await?;
    Ok(json!(qty.unwrap_or(0)))
}

async fn friend_limit(pool: &MySqlPool, id: i64) -> std::result::Result<i64, sqlx::Error> {
    let account_type: Option<i8> = sqlx::query_scalar(
        "SELECT u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(if account_type.unwrap_or(0) >= 1 {
        PREMIUM_FRIEND_LIMIT
    } else {
        FREE_FRIEND_LIMIT
    })
}

async fn friend_count(pool: &MySqlPool, id: i64) -> std::result::Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT COUNT(*) FROM character_friends WHERE character_id=?")
        .bind(id)
        .fetch_one(pool)
        .await
}

async fn is_recruitable_flag(pool: &MySqlPool, id: i64) -> std::result::Result<bool, sqlx::Error> {
    let rec: Option<bool> = sqlx::query_scalar("SELECT recruitable FROM characters WHERE id=?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(rec.unwrap_or(true))
}

async fn paginated_friend_response(
    pool: &MySqlPool,
    id: i64,
    items: Vec<Json>,
    page: i64,
    per_page: i64,
) -> Result {
    let total = items.len() as i64;
    let total_pages = ((total + per_page - 1) / per_page).max(1);
    let current_page = page.clamp(1, total_pages);
    let offset = ((current_page - 1) * per_page) as usize;
    let slice: Vec<Json> = items
        .into_iter()
        .skip(offset)
        .take(per_page as usize)
        .collect();
    let kunai = kunai_count(pool, id).await?;
    let recruitable = is_recruitable_flag(pool, id).await?;
    let count = friend_count(pool, id).await?;
    let limit = friend_limit(pool, id).await?;
    Ok(json!({
        "status": 1,
        "friends": slice,
        "page": format!("{current_page}/{total_pages}"),
        "kunai": kunai,
        "recruitable": recruitable,
        "limit": format!("{count}/{limit}")
    }))
}

async fn friends(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let page = integer(args, 2).max(1);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let rows = sqlx::query(
        "SELECT c.*, u.account_type, cf.is_favorite \
         FROM character_friends cf \
         JOIN characters c ON c.id=cf.friend_id \
         JOIN users u ON u.id=c.user_id \
         WHERE cf.character_id=? \
         ORDER BY cf.is_favorite DESC, cf.id ASC",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    let formatted: Vec<Json> = rows
        .iter()
        .map(|r| {
            let fav: bool = r.try_get("is_favorite").unwrap_or(false);
            format_character_for_friend(r, fav)
        })
        .collect();
    paginated_friend_response(pool, id, formatted, page, FRIENDS_PER_PAGE).await
}

async fn get_favorite(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let page = integer(args, 2).max(1);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let rows = sqlx::query(
        "SELECT c.*, u.account_type \
         FROM character_friends cf \
         JOIN characters c ON c.id=cf.friend_id \
         JOIN users u ON u.id=c.user_id \
         WHERE cf.character_id=? AND cf.is_favorite=1 \
         ORDER BY cf.id ASC",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    let formatted: Vec<Json> = rows
        .iter()
        .map(|r| format_character_for_friend(r, true))
        .collect();
    paginated_friend_response(pool, id, formatted, page, FRIENDS_PER_PAGE).await
}

async fn search(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let query = string(args, 2);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let q_trim = query.trim();
    let rows = if q_trim.is_empty() {
        sqlx::query(
            "SELECT c.*, u.account_type, cf.is_favorite \
             FROM character_friends cf \
             JOIN characters c ON c.id=cf.friend_id \
             JOIN users u ON u.id=c.user_id \
             WHERE cf.character_id=? \
             ORDER BY cf.is_favorite DESC, cf.id ASC",
        )
        .bind(id)
        .fetch_all(pool)
        .await?
    } else if q_trim.chars().all(|c| c.is_ascii_digit()) {
        let target_id = q_trim.parse::<i64>().unwrap_or(0);
        sqlx::query(
            "SELECT c.*, u.account_type, cf.is_favorite \
             FROM character_friends cf \
             JOIN characters c ON c.id=cf.friend_id \
             JOIN users u ON u.id=c.user_id \
             WHERE cf.character_id=? AND (cf.friend_id=? OR c.name LIKE ?) \
             ORDER BY cf.is_favorite DESC, cf.id ASC",
        )
        .bind(id)
        .bind(target_id)
        .bind(format!("%{q_trim}%"))
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT c.*, u.account_type, cf.is_favorite \
             FROM character_friends cf \
             JOIN characters c ON c.id=cf.friend_id \
             JOIN users u ON u.id=c.user_id \
             WHERE cf.character_id=? AND c.name LIKE ? \
             ORDER BY cf.is_favorite DESC, cf.id ASC",
        )
        .bind(id)
        .bind(format!("%{q_trim}%"))
        .fetch_all(pool)
        .await?
    };
    let formatted: Vec<Json> = rows
        .iter()
        .map(|r| {
            let fav: bool = r.try_get("is_favorite").unwrap_or(false);
            format_character_for_friend(r, fav)
        })
        .collect();
    paginated_friend_response(pool, id, formatted, 1, FRIENDS_PER_PAGE).await
}

async fn get_recommendations(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let Some(char_level): Option<i32> =
        sqlx::query_scalar("SELECT level FROM characters WHERE id=?")
            .bind(id)
            .fetch_optional(pool)
            .await?
    else {
        return Ok(json!({"status":0,"result":"Character not found"}));
    };

    let mut excluded: HashSet<i64> = HashSet::new();
    excluded.insert(id);
    let friends: Vec<u64> =
        sqlx::query_scalar("SELECT friend_id FROM character_friends WHERE character_id=?")
            .bind(id)
            .fetch_all(pool)
            .await?;
    for f in friends {
        excluded.insert(f as i64);
    }
    let outgoing: Vec<u64> =
        sqlx::query_scalar("SELECT character_id FROM friend_requests WHERE requester_id=?")
            .bind(id)
            .fetch_all(pool)
            .await?;
    for o in outgoing {
        excluded.insert(o as i64);
    }
    let incoming: Vec<u64> =
        sqlx::query_scalar("SELECT requester_id FROM friend_requests WHERE character_id=?")
            .bind(id)
            .fetch_all(pool)
            .await?;
    for inc in incoming {
        excluded.insert(inc as i64);
    }

    let min_level = (char_level - 15).max(1);
    let max_level = char_level + 15;
    let candidates = sqlx::query(
        "SELECT c.*, u.account_type FROM characters c JOIN users u ON u.id=c.user_id \
         WHERE c.level BETWEEN ? AND ? ORDER BY RAND() LIMIT 50",
    )
    .bind(min_level)
    .bind(max_level)
    .fetch_all(pool)
    .await?;

    let mut selected = Vec::new();
    for r in candidates {
        let cid: u64 = r.try_get("id")?;
        if !excluded.contains(&(cid as i64)) {
            excluded.insert(cid as i64);
            selected.push(format_character_for_friend(&r, false));
            if selected.len() as i64 >= RECOMMENDATIONS_PER_PAGE {
                break;
            }
        }
    }
    if (selected.len() as i64) < RECOMMENDATIONS_PER_PAGE {
        let more = sqlx::query(
            "SELECT c.*, u.account_type FROM characters c JOIN users u ON u.id=c.user_id \
             ORDER BY RAND() LIMIT 50",
        )
        .fetch_all(pool)
        .await?;
        for r in more {
            let cid: u64 = r.try_get("id")?;
            if !excluded.contains(&(cid as i64)) {
                excluded.insert(cid as i64);
                selected.push(format_character_for_friend(&r, false));
                if selected.len() as i64 >= RECOMMENDATIONS_PER_PAGE {
                    break;
                }
            }
        }
    }
    paginated_friend_response(pool, id, selected, 1, RECOMMENDATIONS_PER_PAGE).await
}

async fn add_friend(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let target_id = integer(args, 2);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    if id == target_id {
        return Ok(json!({"status":2,"result":"You cannot add yourself as a friend."}));
    }
    let Some(target_row) = sqlx::query("SELECT id, name FROM characters WHERE id=?")
        .bind(target_id)
        .fetch_optional(pool)
        .await?
    else {
        return Ok(json!({"status":2,"result":"Character not found."}));
    };
    let target_name: String = target_row.try_get("name")?;

    let already: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM character_friends WHERE character_id=? AND friend_id=?",
    )
    .bind(id)
    .bind(target_id)
    .fetch_one(pool)
    .await?;
    if already > 0 {
        return Ok(json!({"status":2,"result":"Already in your friend list."}));
    }

    if friend_count(pool, id).await? >= friend_limit(pool, id).await? {
        return Ok(json!({"status":2,"result":"Your friend list is full."}));
    }

    // Check reverse pending request
    let reverse_id: Option<u64> = sqlx::query_scalar(
        "SELECT id FROM friend_requests WHERE character_id=? AND requester_id=?",
    )
    .bind(id)
    .bind(target_id)
    .fetch_optional(pool)
    .await?;
    if let Some(rev_id) = reverse_id {
        if friend_count(pool, target_id).await? >= friend_limit(pool, target_id).await? {
            return Ok(json!({"status":2,"result":"Target player friend list is full."}));
        }
        let mut tx = pool.begin().await?;
        sqlx::query("DELETE FROM friend_requests WHERE id=?")
            .bind(rev_id)
            .execute(&mut *tx)
            .await?;
        create_friendship_pair(&mut tx, id, target_id).await?;
        tx.commit().await?;
        return Ok(json!({"status":1,"result":format!("You are now friends with {target_name}!")}));
    }

    let existing_req: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM friend_requests WHERE character_id=? AND requester_id=?",
    )
    .bind(target_id)
    .bind(id)
    .fetch_one(pool)
    .await?;
    if existing_req > 0 {
        return Ok(json!({"status":2,"result":"Friend request already sent."}));
    }

    sqlx::query(
        "INSERT INTO friend_requests (character_id,requester_id,created_at,updated_at) VALUES (?,?,NOW(),NOW())",
    )
    .bind(target_id)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(json!({"status":1,"result":format!("Friend request sent to {target_name}.")}))
}

async fn friend_requests(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let page = integer(args, 2).max(1);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let rows = sqlx::query(
        "SELECT c.*, u.account_type \
         FROM friend_requests fr \
         JOIN characters c ON c.id=fr.requester_id \
         JOIN users u ON u.id=c.user_id \
         WHERE fr.character_id=? \
         ORDER BY fr.id DESC",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    let total = rows.len() as i64;
    let total_pages = ((total + REQUESTS_PER_PAGE - 1) / REQUESTS_PER_PAGE).max(1);
    let current_page = page.clamp(1, total_pages);
    let offset = ((current_page - 1) * REQUESTS_PER_PAGE) as usize;
    let list: Vec<Json> = rows
        .iter()
        .skip(offset)
        .take(REQUESTS_PER_PAGE as usize)
        .map(|r| format_character_for_friend(r, false))
        .collect();
    Ok(json!({
        "status": 1,
        "Request": list,
        "page": format!("{current_page}/{total_pages}")
    }))
}

async fn accept_friend(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let requester_id = integer(args, 2);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let Some(req_row_id): Option<u64> = sqlx::query_scalar(
        "SELECT id FROM friend_requests WHERE character_id=? AND requester_id=?",
    )
    .bind(id)
    .bind(requester_id)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(json!({"status":2,"result":"Friend request not found."}));
    };

    if friend_count(pool, id).await? >= friend_limit(pool, id).await? {
        return Ok(json!({"status":2,"result":"Your friend list is full."}));
    }
    if friend_count(pool, requester_id).await? >= friend_limit(pool, requester_id).await? {
        sqlx::query("DELETE FROM friend_requests WHERE id=?")
            .bind(req_row_id)
            .execute(pool)
            .await?;
        return Ok(json!({"status":2,"result":"Requester friend list is full."}));
    }

    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM friend_requests WHERE id=?")
        .bind(req_row_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM friend_requests WHERE character_id=? AND requester_id=?")
        .bind(requester_id)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    create_friendship_pair(&mut tx, id, requester_id).await?;
    tx.commit().await?;
    Ok(json!({"status":1,"result":"Friend request accepted."}))
}

async fn accept_all(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let requests: Vec<(u64, u64)> = sqlx::query_as(
        "SELECT id, requester_id FROM friend_requests WHERE character_id=? ORDER BY id ASC",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    if requests.is_empty() {
        return Ok(json!({"status":2,"result":"No pending friend requests."}));
    }

    let my_limit = friend_limit(pool, id).await?;
    let mut my_count = friend_count(pool, id).await?;
    let mut accepted = 0;

    let mut tx = pool.begin().await?;
    for (req_id, req_char_id) in requests {
        if my_count >= my_limit {
            break;
        }
        let req_char_id_i64 = req_char_id as i64;
        let their_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM character_friends WHERE character_id=?")
                .bind(req_char_id_i64)
                .fetch_one(&mut *tx)
                .await?;
        let their_acct: Option<i8> = sqlx::query_scalar(
            "SELECT u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=?",
        )
        .bind(req_char_id_i64)
        .fetch_optional(&mut *tx)
        .await?;
        if their_acct.is_none() {
            sqlx::query("DELETE FROM friend_requests WHERE id=?")
                .bind(req_id)
                .execute(&mut *tx)
                .await?;
            continue;
        }
        let their_limit = if their_acct.unwrap_or(0) >= 1 {
            PREMIUM_FRIEND_LIMIT
        } else {
            FREE_FRIEND_LIMIT
        };
        if their_count >= their_limit {
            continue;
        }
        sqlx::query("DELETE FROM friend_requests WHERE id=?")
            .bind(req_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM friend_requests WHERE character_id=? AND requester_id=?")
            .bind(req_char_id_i64)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        create_friendship_pair(&mut tx, id, req_char_id_i64).await?;
        my_count += 1;
        accepted += 1;
    }
    tx.commit().await?;
    Ok(json!({"status":1,"result":format!("Accepted {accepted} friend request(s).")}))
}

async fn remove_all(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    sqlx::query("DELETE FROM friend_requests WHERE character_id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(json!({"status":1,"result":"All friend requests declined."}))
}

async fn create_friendship_pair(
    tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
    char_a: i64,
    char_b: i64,
) -> std::result::Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT IGNORE INTO character_friends (character_id,friend_id,is_favorite,created_at,updated_at) VALUES (?,?,0,NOW(),NOW())",
    )
    .bind(char_a)
    .bind(char_b)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT IGNORE INTO character_friends (character_id,friend_id,is_favorite,created_at,updated_at) VALUES (?,?,0,NOW(),NOW())",
    )
    .bind(char_b)
    .bind(char_a)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn set_favorite(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let friend_id = integer(args, 2);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let res = sqlx::query(
        "UPDATE character_friends SET is_favorite=1, updated_at=NOW() WHERE character_id=? AND friend_id=?",
    )
    .bind(id)
    .bind(friend_id)
    .execute(pool)
    .await?;
    if res.rows_affected() == 0 {
        return Ok(json!({"status":2,"result":"Friend not found."}));
    }
    friends(pool, &[json!(id), json!(key), json!(1)]).await
}

async fn remove_favorite(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let friend_id = integer(args, 2);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let res = sqlx::query(
        "UPDATE character_friends SET is_favorite=0, updated_at=NOW() WHERE character_id=? AND friend_id=?",
    )
    .bind(id)
    .bind(friend_id)
    .execute(pool)
    .await?;
    if res.rows_affected() == 0 {
        return Ok(json!({"status":2,"result":"Friend not found."}));
    }
    friends(pool, &[json!(id), json!(key), json!(1)]).await
}

async fn remove_friend(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let friend_id = integer(args, 2);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM character_friends WHERE character_id=? AND friend_id=?",
    )
    .bind(id)
    .bind(friend_id)
    .fetch_one(pool)
    .await?;
    if exists == 0 {
        return Ok(json!({"status":2,"result":"Friend not found."}));
    }
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM character_friends WHERE character_id=? AND friend_id=?")
        .bind(id)
        .bind(friend_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM character_friends WHERE character_id=? AND friend_id=?")
        .bind(friend_id)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM character_recruits WHERE character_id=? AND recruit_id IN (?,?)")
        .bind(id)
        .bind(friend_id.to_string())
        .bind(format!("char_{friend_id}"))
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM character_recruits WHERE character_id=? AND recruit_id IN (?,?)")
        .bind(friend_id)
        .bind(id.to_string())
        .bind(format!("char_{id}"))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    friends(pool, &[json!(id), json!(key), json!(1)]).await
}

async fn unfriend_all(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let friend_ids: Vec<u64> =
        sqlx::query_scalar("SELECT friend_id FROM character_friends WHERE character_id=?")
            .bind(id)
            .fetch_all(pool)
            .await?;
    if !friend_ids.is_empty() {
        let mut tx = pool.begin().await?;
        sqlx::query("DELETE FROM character_friends WHERE character_id=? OR friend_id=?")
            .bind(id)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        for fid in friend_ids {
            sqlx::query(
                "DELETE FROM character_recruits WHERE character_id=? AND recruit_id IN (?,?)",
            )
            .bind(id)
            .bind(fid.to_string())
            .bind(format!("char_{fid}"))
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                "DELETE FROM character_recruits WHERE character_id=? AND recruit_id IN (?,?)",
            )
            .bind(fid)
            .bind(id.to_string())
            .bind(format!("char_{id}"))
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
    }
    friends(pool, &[json!(id), json!(key), json!(1)]).await
}

async fn recruitable(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let Some(curr): Option<bool> =
        sqlx::query_scalar("SELECT recruitable FROM characters WHERE id=?")
            .bind(id)
            .fetch_optional(pool)
            .await?
    else {
        return Ok(json!({"status":0,"result":"Character not found"}));
    };
    let next = !curr;
    sqlx::query("UPDATE characters SET recruitable=?, updated_at=NOW() WHERE id=?")
        .bind(next)
        .bind(id)
        .execute(pool)
        .await?;
    friends(pool, &[json!(id), json!(key), json!(1)]).await
}

async fn recruit_friend(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let friend_id = integer(args, 2);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let is_friend: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM character_friends WHERE character_id=? AND friend_id=?",
    )
    .bind(id)
    .bind(friend_id)
    .fetch_one(pool)
    .await?;
    if is_friend == 0 {
        return Ok(
            json!({"status":2,"result":"You can only recruit characters from your friend list."}),
        );
    }
    let Some(friend_row) = sqlx::query("SELECT name, recruitable FROM characters WHERE id=?")
        .bind(friend_id)
        .fetch_optional(pool)
        .await?
    else {
        return Ok(json!({"status":2,"result":"Friend character not found."}));
    };
    let rec_flag: bool = friend_row.try_get("recruitable").unwrap_or(true);
    if !rec_flag {
        return Ok(
            json!({"status":2,"result":"This friend does not allow recruitment right now."}),
        );
    }
    let friend_name: String = friend_row.try_get("name")?;
    let recruit_token = format!("char_{friend_id}");
    let already: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM character_recruits WHERE character_id=? AND recruit_id IN (?,?)",
    )
    .bind(id)
    .bind(&recruit_token)
    .bind(friend_id.to_string())
    .fetch_one(pool)
    .await?;
    if already > 0 {
        return Ok(json!({"status":2,"result":"This friend is already in your squad."}));
    }
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM character_recruits WHERE character_id=?")
            .bind(id)
            .fetch_one(pool)
            .await?;
    if count >= 2 {
        return Ok(json!({"status":2,"result":"You cannot recruit more teammates."}));
    }
    sqlx::query(
        "INSERT INTO character_recruits (character_id,recruit_id,created_at,updated_at) VALUES (?,?,NOW(),NOW())",
    )
    .bind(id)
    .bind(&recruit_token)
    .execute(pool)
    .await?;
    Ok(json!({
        "status": 1,
        "result": format!("{friend_name} has been recruited to your squad!")
    }))
}

async fn start_berantem(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let friend_id = integer(args, 1);
    let client_hash = string(args, 2);
    let key = string(args, 3);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let expected = authentication::sha256_hex(&format!("{id}{friend_id}{key}"));
    if !client_hash.eq_ignore_ascii_case(&expected) {
        return Ok(json!({"status":2,"result":"Invalid battle request."}));
    }
    let is_friend: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM character_friends WHERE character_id=? AND friend_id=?",
    )
    .bind(id)
    .bind(friend_id)
    .fetch_one(pool)
    .await?;
    if is_friend == 0 {
        return Ok(json!({"status":2,"result":"You can only spar with friends."}));
    }
    let Some(friend_row) = sqlx::query(
        "SELECT c.*, u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=?",
    )
    .bind(friend_id)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(json!({"status":2,"result":"Opponent character not found."}));
    };

    let battle_code = authentication::random_hex()
        .unwrap_or_else(|_| "0123456789abcdef".into())
        .chars()
        .take(12)
        .collect::<String>();
    let response_hash = authentication::sha256_hex(&format!("{battle_code}{id}{friend_id}"));

    let gender: i8 = friend_row.try_get("gender").unwrap_or(0);
    let suffix = if gender == 0 { "_0" } else { "_1" };
    let hair_raw: Option<String> = friend_row.try_get("hair_style").ok().flatten();
    let hair = match hair_raw.as_deref() {
        Some(h) if !h.is_empty() && h.chars().all(|c| c.is_ascii_digit()) => {
            format!("hair_{:0>2}{suffix}", h)
        }
        Some(h) if !h.is_empty() => h.to_string(),
        _ => format!("hair_01{suffix}"),
    };
    let weapon = friend_row
        .try_get::<Option<String>, _>("equipment_weapon")?
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "wpn_01".into());
    let back_item = friend_row
        .try_get::<Option<String>, _>("equipment_back")?
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "back_01".into());
    let clothing = friend_row
        .try_get::<Option<String>, _>("equipment_clothing")?
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("set_01{suffix}"));
    let accessory = friend_row
        .try_get::<Option<String>, _>("equipment_accessory")?
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "accessory_01".into());
    let hair_color = friend_row
        .try_get::<Option<String>, _>("hair_color")?
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "0|0".into());
    let skin_color = friend_row
        .try_get::<Option<String>, _>("skin_color")?
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "0|0".into());
    let skills = friend_row
        .try_get::<Option<String>, _>("equipment_skills")?
        .unwrap_or_default();
    let senjutsu_skills = friend_row
        .try_get::<Option<String>, _>("senjutsu_equipped_skills")?
        .unwrap_or_default();

    let mut pet_data = json!({});
    if let Some(pet_ref) = friend_row
        .try_get::<Option<String>, _>("equipment_pet")?
        .filter(|s| !s.is_empty() && s != "0")
    {
        let pet_row = if pet_ref.chars().all(|c| c.is_ascii_digit()) {
            sqlx::query("SELECT * FROM character_pets WHERE character_id=? AND id=?")
                .bind(friend_id)
                .bind(pet_ref.parse::<i64>().unwrap_or(0))
                .fetch_optional(pool)
                .await?
        } else {
            sqlx::query(
                "SELECT * FROM character_pets WHERE character_id=? AND pet_id=? ORDER BY id LIMIT 1",
            )
            .bind(friend_id)
            .bind(&pet_ref)
            .fetch_optional(pool)
            .await?
        };
        if let Some(pet) = pet_row {
            let pet_swf: String = pet.try_get("pet_id")?;
            let config_name: Option<String> =
                sqlx::query_scalar("SELECT name FROM pets WHERE pet_id=?")
                    .bind(&pet_swf)
                    .fetch_optional(pool)
                    .await?;
            let pet_name = pet
                .try_get::<Option<String>, _>("name")?
                .filter(|s| !s.is_empty())
                .or(config_name)
                .unwrap_or_else(|| "Pet".into());
            pet_data = json!({
                "pet_id": pet.try_get::<u64, _>("id")?,
                "pet_name": pet_name,
                "pet_level": pet.try_get::<i32, _>("level")?,
                "pet_swf": pet_swf,
                "pet_skills": "1,1,1,1,1,1",
                "pet_mp": pet.try_get::<u32, _>("maturity_points").unwrap_or(0)
            });
        }
    }

    let f_uid = friend_row.try_get::<u64, _>("id")?;
    let f_name: String = friend_row.try_get("name")?;
    let f_level: i32 = friend_row.try_get("level")?;
    let f_rank: i8 = friend_row.try_get("rank")?;

    Ok(json!({
        "status": 1,
        "code": battle_code,
        "hash": response_hash,
        "friend": {
            "character_data": {
                "character_id": f_uid,
                "character_name": f_name,
                "character_level": f_level,
                "character_xp": friend_row.try_get::<i64, _>("xp").unwrap_or(0),
                "character_gender": gender,
                "character_rank": f_rank,
                "character_merit": 0,
                "character_prestige": friend_row.try_get::<i32, _>("prestige").unwrap_or(0),
                "character_element_1": friend_row.try_get::<i8, _>("element_1").unwrap_or(0),
                "character_element_2": friend_row.try_get::<i8, _>("element_2").unwrap_or(0),
                "character_element_3": friend_row.try_get::<i8, _>("element_3").unwrap_or(0),
                "character_talent_1": friend_row.try_get::<Option<String>, _>("talent_1")?,
                "character_talent_2": friend_row.try_get::<Option<String>, _>("talent_2")?,
                "character_talent_3": friend_row.try_get::<Option<String>, _>("talent_3")?,
                "character_gold": friend_row.try_get::<i32, _>("gold").unwrap_or(0),
                "character_tp": friend_row.try_get::<i32, _>("tp").unwrap_or(0),
                "character_ss": friend_row.try_get::<i32, _>("ss").unwrap_or(0),
                "character_class": friend_row.try_get::<Option<String>, _>("class")?,
                "character_senjutsu": friend_row.try_get::<Option<String>, _>("senjutsu_type")?,
                "character_pvp_points": friend_row.try_get::<i32, _>("pvp_points").unwrap_or(0)
            },
            "character_points": {
                "atrrib_wind": friend_row.try_get::<i32, _>("point_wind").unwrap_or(0),
                "atrrib_fire": friend_row.try_get::<i32, _>("point_fire").unwrap_or(0),
                "atrrib_lightning": friend_row.try_get::<i32, _>("point_lightning").unwrap_or(0),
                "atrrib_water": friend_row.try_get::<i32, _>("point_water").unwrap_or(0),
                "atrrib_earth": friend_row.try_get::<i32, _>("point_earth").unwrap_or(0),
                "atrrib_free": friend_row.try_get::<i32, _>("point_free").unwrap_or(0)
            },
            "character_sets": {
                "weapon": weapon,
                "back_item": back_item,
                "clothing": clothing,
                "hairstyle": hair,
                "accessory": accessory,
                "face": format!("face_01{suffix}"),
                "hair_color": hair_color,
                "skin_color": skin_color,
                "skills": skills,
                "senjutsu_skills": senjutsu_skills,
                "talent_skills": friend_row.try_get::<Option<String>, _>("talent_skills")?.unwrap_or_default(),
                "pet": friend_row.try_get::<Option<String>, _>("equipment_pet")?.unwrap_or_default()
            },
            "pet_data": pet_data,
            "clan": null
        }
    }))
}

async fn end_berantem(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let battle_code = string(args, 1);
    let client_hash = string(args, 2);
    let key = string(args, 3);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let expected = authentication::sha256_hex(&format!("{id}{battle_code}{key}"));
    if !client_hash.eq_ignore_ascii_case(&expected) {
        return Ok(json!({"status":2,"result":"Invalid battle finish hash."}));
    }
    let Some(char_row) = sqlx::query("SELECT level, xp FROM characters WHERE id=?")
        .bind(id)
        .fetch_optional(pool)
        .await?
    else {
        return Ok(json!({"status":0,"result":"Character not found"}));
    };
    let level: i32 = char_row.try_get("level")?;
    let xp: i64 = char_row.try_get("xp")?;

    let mut tx = pool.begin().await?;
    rewards::give_item(&mut tx, id, FRIENDSHIP_KUNAI, 1).await?;
    tx.commit().await?;

    Ok(json!({
        "status": 1,
        "result": [0, 0, [format!("{FRIENDSHIP_KUNAI}:1")]],
        "level": level,
        "xp": xp,
        "level_up": false
    }))
}

async fn get_shop_catalog(pool: &MySqlPool) -> std::result::Result<Vec<Json>, sqlx::Error> {
    let cfg_val: Option<Json> =
        sqlx::query_scalar("SELECT `value` FROM game_configs WHERE `key`='friendship_shop'")
            .fetch_optional(pool)
            .await?;
    if let Some(Json::Array(arr)) = cfg_val
        && !arr.is_empty()
    {
        return Ok(arr
            .into_iter()
            .enumerate()
            .map(|(idx, entry)| {
                let id_val = entry.get("id").and_then(Json::as_i64).unwrap_or(idx as i64);
                let item = entry
                    .get("item")
                    .and_then(Json::as_str)
                    .unwrap_or("")
                    .to_string();
                let price = entry.get("price").and_then(Json::as_i64).unwrap_or(1);
                json!({
                    "id": id_val,
                    "item": item,
                    "price": price
                })
            })
            .collect());
    }
    let rows = sqlx::query(
        "SELECT item_id, price_tokens, price_gold FROM items ORDER BY item_id LIMIT 10",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .enumerate()
        .map(|(idx, r)| {
            let item_id: String = r.try_get("item_id").unwrap_or_default();
            let pt: i32 = r.try_get("price_tokens").unwrap_or(0);
            let pg: i32 = r.try_get("price_gold").unwrap_or(1000);
            let price = if pt > 0 {
                pt
            } else {
                ((pg.max(1000) + 999) / 1000).max(1)
            };
            json!({
                "id": idx as i64,
                "item": item_id,
                "price": price
            })
        })
        .collect())
}

async fn get_items(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let catalog = get_shop_catalog(pool).await?;
    Ok(json!({
        "status": 1,
        "items": catalog
    }))
}

async fn buy_item(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let exchange_id = integer(args, 2);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let catalog = get_shop_catalog(pool).await?;
    let Some(entry) = catalog
        .into_iter()
        .find(|e| e.get("id").and_then(Json::as_i64) == Some(exchange_id))
    else {
        return Ok(json!({"status":2,"result":"Item not found."}));
    };
    let item_spec = entry
        .get("item")
        .and_then(Json::as_str)
        .unwrap_or("")
        .to_string();
    let price = entry.get("price").and_then(Json::as_i64).unwrap_or(1) as i32;
    let (item_id, qty) = match item_spec.split_once(':') {
        Some((i, q)) => (i.to_string(), q.parse::<i32>().unwrap_or(1).max(1)),
        None => (item_spec.clone(), 1),
    };

    let mut tx = pool.begin().await?;
    let kunai_row = sqlx::query(
        "SELECT id, quantity FROM character_items WHERE character_id=? AND item_id=? FOR UPDATE",
    )
    .bind(id)
    .bind(FRIENDSHIP_KUNAI)
    .fetch_optional(&mut *tx)
    .await?;
    let current_kunai = kunai_row
        .as_ref()
        .and_then(|r| r.try_get::<i32, _>("quantity").ok())
        .unwrap_or(0);
    if current_kunai < price {
        return Ok(json!({"status":2,"result":"Not enough Friendship Kunai."}));
    }
    let remaining_kunai = current_kunai - price;
    if let Some(r) = kunai_row {
        let row_id: u64 = r.try_get("id")?;
        sqlx::query("UPDATE character_items SET quantity=?, updated_at=NOW() WHERE id=?")
            .bind(remaining_kunai)
            .bind(row_id)
            .execute(&mut *tx)
            .await?;
    }
    rewards::give_item(&mut tx, id, &item_id, i64::from(qty)).await?;
    tx.commit().await?;

    Ok(json!({
        "status": 1,
        "reward": item_spec,
        "kunai": remaining_kunai
    }))
}
