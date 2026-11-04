use crate::account::{guard, integer, string};
use persistence::{MySqlPool, Row, row_json, sqlx};
use serde_json::{Value as Json, json};
type Result = std::result::Result<Json, sqlx::Error>;

pub async fn list(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(error) = guard(pool, id, &string(args, 1), false).await? {
        return Ok(error);
    }
    let user = sqlx::query("SELECT account_type,tokens FROM users WHERE id=?")
        .bind(id)
        .fetch_one(pool)
        .await?;
    let mut characters = Vec::new();
    for row in sqlx::query("SELECT * FROM characters WHERE user_id=? ORDER BY id")
        .bind(id)
        .fetch_all(pool)
        .await?
    {
        let c = row_json(&row)?;
        let mut info = json!({"char_id":c["id"],"acc_id":id});
        for key in [
            "name",
            "level",
            "xp",
            "gender",
            "rank",
            "prestige",
            "element_1",
            "element_2",
            "element_3",
            "talent_1",
            "talent_2",
            "talent_3",
            "gold",
            "tp",
        ] {
            let mut value = c[key].clone();
            if (key.starts_with("talent_") || key == "element_2" || key == "element_3")
                && (value == 0 || value == "" || value == "0")
            {
                value = Json::Null;
            }
            info[format!("character_{key}")] = value;
        }
        characters.push(info);
    }
    Ok(
        json!({"status":1,"error":0,"account_type":user.try_get::<i8,_>("account_type")?,"emblem_duration":-1,"tokens":user.try_get::<i32,_>("tokens")?,"total_characters":characters.len(),"account_data":characters}),
    )
}

pub async fn create(pool: &MySqlPool, args: &[Json]) -> Result {
    let uid = integer(args, 0);
    let name = string(args, 2);
    let gender = integer(args, 3);
    let element = integer(args, 4);
    // Source registration omitted its service guard. Ownership is enforced explicitly.
    if let Some(error) = crate::account::registration_guard(pool, uid, &string(args, 1)).await? {
        return Ok(error);
    }
    if name.len() < 2 {
        return Ok(json!({"status":2,"error":"Character name too short."}));
    }
    if name.len() > 255 || !(0..=1).contains(&gender) || !(1..=5).contains(&element) {
        return Ok(json!({"status":2,"error":"Invalid character details."}));
    }
    let mut tx = pool.begin().await?;
    let user = sqlx::query("SELECT account_type FROM users WHERE id=? FOR UPDATE")
        .bind(uid)
        .fetch_one(&mut *tx)
        .await?;
    let taken: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM characters WHERE name=?")
        .bind(&name)
        .fetch_one(&mut *tx)
        .await?;
    if taken > 0 {
        return Ok(json!({"status":2,"error":"Character name already taken."}));
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM characters WHERE user_id=?")
        .bind(uid)
        .fetch_one(&mut *tx)
        .await?;
    let premium = user.try_get::<i8, _>("account_type")? == 1;
    if count >= if premium { 6 } else { 1 } {
        return Ok(
            json!({"status":2,"result":if premium {"You have reached the maximum limit of 6 characters."} else {"Free users can only create 1 character. Upgrade to Premium for 6 slots!"}}),
        );
    }
    let skill = basic_skill(element);
    let suffix = gender.to_string();
    let hair = format!("hair_{:0>2}_{suffix}", string(args, 6));
    let clothing = format!("set_01_{suffix}");
    let id=sqlx::query("INSERT INTO characters (user_id,name,gender,element_1,hair_style,hair_color,point_free,gold,equipment_weapon,equipment_back,equipment_clothing,equipment_accessory,equipment_skills,created_at,updated_at) VALUES (?,?,?,?,?,?,1,1000,'wpn_01','back_01',?,'accessory_01',?,NOW(),NOW())")
        .bind(uid).bind(name).bind(gender).bind(element).bind(hair).bind(string(args,5)).bind(&clothing).bind(skill).execute(&mut *tx).await?.last_insert_id();
    sqlx::query("INSERT INTO character_skills (character_id,skill_id,created_at,updated_at) VALUES (?,?,NOW(),NOW())").bind(id).bind(skill).execute(&mut *tx).await?;
    for (item, category) in [
        ("wpn_01", "weapon"),
        ("back_01", "back"),
        ("accessory_01", "accessory"),
        (&clothing, "set"),
    ] {
        sqlx::query("INSERT INTO character_items (character_id,item_id,quantity,category,created_at,updated_at) VALUES (?,?,1,?,NOW(),NOW())").bind(id).bind(item).bind(category).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(json!({"status":1}))
}
fn basic_skill(element: i64) -> &'static str {
    match element {
        1 => "skill_13",
        2 => "skill_10",
        4 => "skill_12",
        5 => "skill_09",
        _ => "skill_01",
    }
}
fn default(c: &Json, key: &str, fallback: &str) -> String {
    c[key]
        .as_str()
        .filter(|s| !s.is_empty() && *s != "0")
        .unwrap_or(fallback)
        .to_owned()
}

pub async fn load(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(error) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(error);
    }
    load_character(pool, id).await
}

async fn load_character(pool: &MySqlPool, id: i64) -> Result {
    let row=sqlx::query("SELECT c.*,u.account_type,u.tokens AS account_tokens FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=?").bind(id).fetch_one(pool).await?;
    let c = row_json(&row)?;
    let gender = c["gender"].as_i64().unwrap_or(0);
    let mut data = json!({"character_id":id,"character_merit":0});
    for key in [
        "name",
        "level",
        "xp",
        "gender",
        "rank",
        "prestige",
        "element_1",
        "element_2",
        "element_3",
        "talent_1",
        "talent_2",
        "talent_3",
        "gold",
        "tp",
        "ss",
        "class",
        "pvp_points",
    ] {
        data[format!("character_{key}")] = c[key].clone();
    }
    data["character_senjutsu"] = c["senjutsu_type"].clone();
    let cap = match c["rank"].as_i64().unwrap_or(1) {
        1 => 20,
        3 => 40,
        5 => 60,
        7 => 80,
        _ => i64::MAX,
    };
    data["character_level"] = json!(c["level"].as_i64().unwrap_or(1).min(cap));
    let mut points = json!({});
    for key in ["wind", "fire", "lightning", "water", "earth", "free"] {
        points[format!("atrrib_{key}")] = c[format!("point_{key}")].clone();
    }
    let mut inventory = json!({});
    let items = sqlx::query(
        "SELECT item_id,quantity,category FROM character_items WHERE character_id=? ORDER BY id",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    for (field, category) in [
        ("char_weapons", "weapon"),
        ("char_back_items", "back"),
        ("char_accessories", "accessory"),
        ("char_sets", "set"),
        ("char_hairs", "hair"),
        ("char_materials", "material"),
        ("char_items", "item"),
        ("char_essentials", "essential"),
        ("char_animations", "animation"),
    ] {
        let parts: Vec<String> = items
            .iter()
            .filter(|r| {
                r.try_get::<String, _>("category")
                    .is_ok_and(|v| v == category)
            })
            .map(|r| {
                let item: String = r.get("item_id");
                if category == "hair" || category == "animation" {
                    item
                } else {
                    format!("{item}:{}", r.get::<i32, _>("quantity"))
                }
            })
            .collect();
        inventory[field] = json!(parts.join(","));
    }
    let skills: Vec<String> = sqlx::query_scalar(
        "SELECT skill_id FROM character_skills WHERE character_id=? ORDER BY id",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    inventory["char_skills"] = json!(skills.join(","));
    inventory["char_talent_skills"] = json!(default(&c, "talent_skills", ""));
    inventory["char_senjutsu_skills"] = json!(default(&c, "senjutsu_skills", ""));
    let hair = default(&c, "hair_style", &format!("hair_01_{gender}"));
    let hair = if hair.parse::<u32>().is_ok() {
        format!("hair_{hair:0>2}_{gender}")
    } else {
        hair
    };
    let sets = json!({"weapon":default(&c,"equipment_weapon","wpn_01"),"back_item":default(&c,"equipment_back","back_01"),"accessory":default(&c,"equipment_accessory","accessory_01"),"hairstyle":hair,"clothing":default(&c,"equipment_clothing",&format!("set_01_{gender}")),"skills":default(&c,"equipment_skills",basic_skill(c["element_1"].as_i64().unwrap_or(0))),"senjutsu_skills":default(&c,"senjutsu_equipped_skills",""),"hair_color":default(&c,"hair_color","0|0"),"skin_color":default(&c,"skin_color","null|null"),"face":format!("face_01_{gender}"),"pet":default(&c,"equipment_pet",""),"anims":[]});
    let unread: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM character_mails WHERE character_id=? AND viewed=0",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    let pets = crate::pets::list_data(pool, id).await?;
    let active_pet = pets
        .iter()
        .find(|p| {
            p["pet_id"].as_u64().map(|id| id.to_string()).as_deref() == c["equipment_pet"].as_str()
        })
        .cloned()
        .unwrap_or_else(|| json!([]));
    let pet_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM character_pets WHERE character_id=?")
            .bind(id)
            .fetch_one(pool)
            .await?;
    Ok(
        json!({"status":1,"error":0,"announcements":"","account_type":c["account_type"],"tokens":c["account_tokens"],"emblem_duration":-1,"events":[],"has_unread_mails":unread>0,"pet_count":pet_count,"character_data":data,"character_points":points,"character_slots":{"weapons":100,"back_items":100,"accessories":100,"hairstyles":100,"clothing":100},"character_sets":sets,"character_inventory":inventory,"features":["pvp"],"recruiters":[],"recruit_data":[],"pet_data":active_pet,"clan":{"ramen":0,"hot_spring":0,"temple":0,"training_hall":0}}),
    )
}

pub async fn info(pool: &MySqlPool, args: &[Json]) -> Result {
    let caller = integer(args, 0);
    let target = integer(args, 2);
    if let Some(error) = guard(pool, caller, &string(args, 1), true).await? {
        return Ok(error);
    }
    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM characters WHERE id=?")
        .bind(target)
        .fetch_one(pool)
        .await?;
    if exists == 0 {
        return Ok(json!({"status":0,"error":"Character not found"}));
    }
    let mut data = load_character(pool, target).await?;
    let friends: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM character_friends WHERE character_id=? AND friend_id=?",
    )
    .bind(caller)
    .bind(target)
    .fetch_one(pool)
    .await?;
    data["friend"] = json!(friends > 0);
    data["events"] = json!({});
    data["character_data"]["character_pvp_points"] = json!(0);
    let talents = sqlx::query(
        "SELECT skill_id,level FROM character_talents WHERE character_id=? ORDER BY id",
    )
    .bind(target)
    .fetch_all(pool)
    .await?;
    let talent_skills = talents
        .iter()
        .map(|r| {
            format!(
                "{}:{}",
                r.get::<String, _>("skill_id"),
                r.get::<i32, _>("level")
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    data["character_inventory"] = json!({"char_talent_skills":talent_skills,"char_senjutsu_skills":data["character_inventory"]["char_senjutsu_skills"]});
    for key in ["announcements", "tokens", "pet_count"] {
        data.as_object_mut().unwrap().remove(key);
    }
    Ok(data)
}

pub async fn equip(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(error) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(error);
    }
    let mut tx = pool.begin().await?;
    // Lock the character before checking inventory, so concurrent equipment/economy writes serialize.
    sqlx::query("SELECT id FROM characters WHERE id=? FOR UPDATE")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    for index in 2..6 {
        let item = string(args, index);
        let owned:i64=sqlx::query_scalar("SELECT COUNT(*) FROM character_items WHERE character_id=? AND item_id=? AND quantity>0").bind(id).bind(&item).fetch_one(&mut *tx).await?;
        if !item.is_empty() && owned == 0 {
            return Ok(json!({"status":2,"error":"Item is not owned"}));
        }
    }
    sqlx::query("UPDATE characters SET equipment_weapon=?,equipment_back=?,equipment_clothing=?,equipment_accessory=?,hair_style=?,hair_color=?,skin_color=?,updated_at=NOW() WHERE id=?")
        .bind(string(args,2)).bind(string(args,3)).bind(string(args,4)).bind(string(args,5)).bind(string(args,6)).bind(string(args,7)).bind(string(args,8)).bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(json!({"status":1}))
}
pub async fn equip_skills(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(error) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(error);
    }
    let skills = string(args, 2);
    if skills.len() > 4096 || skills.split(',').count() > 8 {
        return Ok(json!({"status":2,"error":"Invalid skill set"}));
    }
    for skill in skills.split(',').filter(|s| !s.is_empty()) {
        let owned: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM character_skills WHERE character_id=? AND skill_id=?",
        )
        .bind(id)
        .bind(skill)
        .fetch_one(pool)
        .await?;
        if owned == 0 {
            return Ok(json!({"status":2,"error":"Skill is not owned"}));
        }
    }
    sqlx::query("UPDATE characters SET equipment_skills=?,updated_at=NOW() WHERE id=?")
        .bind(skills)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(json!({"status":1}))
}

pub async fn set_points(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    if let Some(error) = guard(pool, id, &string(args, 1), true).await? {
        return Ok(error);
    }
    let invalid = || json!({"status":2,"error":"Invalid attribute points"});
    if args.len() != 8 {
        return Ok(invalid());
    }
    let mut points = [0i64; 6];
    for (index, point) in points.iter_mut().enumerate() {
        let value = &args[index + 2];
        let number = value
            .as_i64()
            .or_else(|| {
                value
                    .as_f64()
                    .filter(|n| {
                        n.is_finite() && n.fract() == 0.0 && *n >= 0.0 && *n <= f64::from(i32::MAX)
                    })
                    .map(|n| n as i64)
            })
            .or_else(|| value.as_str()?.parse().ok());
        let Some(number) = number.filter(|n| (0..=i64::from(i32::MAX)).contains(n)) else {
            return Ok(invalid());
        };
        *point = number;
    }
    let mut tx = pool.begin().await?;
    let level: i32 = sqlx::query_scalar("SELECT level FROM characters WHERE id=? FOR UPDATE")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    // UI_Profile.getAttributePoints: free = level - the five allocated attributes.
    // Check the full budget against the durable level, including after a mission.
    if points.iter().sum::<i64>() != i64::from(level) {
        return Ok(invalid());
    }
    sqlx::query("UPDATE characters SET point_wind=?,point_fire=?,point_lightning=?,point_water=?,point_earth=?,point_free=?,updated_at=NOW() WHERE id=?")
        .bind(points[0]).bind(points[1]).bind(points[2]).bind(points[3]).bind(points[4]).bind(points[5]).bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(json!({"status":1,"error":0}))
}

// -----------------------------------------------------------------------------
// AccountService & DevToolsService methods on Character.*
// -----------------------------------------------------------------------------

pub async fn delete_character(pool: &MySqlPool, args: &[Json]) -> Result {
    let char_id = integer(args, 0);
    let session_key = string(args, 1);
    let username = string(args, 2);
    let password = string(args, 3);
    if let Some(error) = guard(pool, char_id, &session_key, true).await? {
        return Ok(error);
    }
    let Some(user) = sqlx::query("SELECT id, password FROM users WHERE username=? LIMIT 1")
        .bind(&username)
        .fetch_optional(pool)
        .await?
    else {
        return Ok(json!({"status": 2, "result": "Invalid password!"}));
    };
    let hash: String = user.try_get("password")?;
    let valid =
        tokio::task::spawn_blocking(move || authentication::verify_password(&password, &hash))
            .await
            .unwrap_or(false);
    if !valid {
        return Ok(json!({"status": 2, "result": "Invalid password!"}));
    }
    let user_id: u64 = user.try_get("id")?;
    let res = sqlx::query("DELETE FROM characters WHERE id=? AND user_id=?")
        .bind(char_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    if res.rows_affected() == 0 {
        return Ok(json!({"status": 2, "result": "Character not found!"}));
    }
    Ok(json!({"status": 1}))
}

pub async fn convert_tokens_to_gold(pool: &MySqlPool, args: &[Json]) -> Result {
    // Params: [sessionKey, charId, tokenAmount]
    let session_key = string(args, 0);
    let char_id = integer(args, 1);
    let amount = integer(args, 2);
    if let Some(error) = guard(pool, char_id, &session_key, true).await? {
        return Ok(error);
    }
    if amount < 1 {
        return Ok(json!({"status": 2, "result": "Please Insert At Least 1 Token!"}));
    }
    if amount > 1_000_000 {
        return Ok(json!({"status": 2, "result": "Maximum conversion is 1,000,000 tokens."}));
    }
    let mut tx = pool.begin().await?;
    let Some(row) = sqlx::query(
        "SELECT c.gold, c.user_id, u.tokens FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE",
    )
    .bind(char_id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(json!({"status": 0, "error": "Character not found."}));
    };
    let tokens = i64::from(row.try_get::<i32, _>("tokens")?);
    if tokens < amount {
        return Ok(json!({"status": 2, "result": "Not enough tokens!"}));
    }
    let new_tokens = i32::try_from(tokens - amount).unwrap_or(0);
    let current_gold = i64::from(row.try_get::<i32, _>("gold")?);
    let new_gold = i32::try_from(
        current_gold
            .saturating_add(amount.saturating_mul(1000))
            .min(i64::from(i32::MAX)),
    )
    .unwrap_or(i32::MAX);
    let user_id: u64 = row.try_get("user_id")?;

    sqlx::query("UPDATE users SET tokens=?, updated_at=NOW() WHERE id=?")
        .bind(new_tokens)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE characters SET gold=?, updated_at=NOW() WHERE id=?")
        .bind(new_gold)
        .bind(char_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(json!({
        "status": 1,
        "character_gold": new_gold,
        "account_tokens": new_tokens
    }))
}

pub async fn rename_character(pool: &MySqlPool, args: &[Json]) -> Result {
    let first_str = string(args, 0);
    let second_str = string(args, 1);
    let (char_id, session_key) =
        if first_str.len() == 32 && args.get(1).is_some_and(|v| v.is_number()) {
            (integer(args, 1), first_str)
        } else {
            (integer(args, 0), second_str)
        };
    let new_name = string(args, 2).trim().to_owned();
    if let Some(error) = guard(pool, char_id, &session_key, true).await? {
        return Ok(error);
    }
    if new_name.len() < 2 {
        return Ok(json!({"status": 2, "error": "Character name too short."}));
    }
    let mut tx = pool.begin().await?;
    let Some(row) = sqlx::query(
        "SELECT c.name, u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE",
    )
    .bind(char_id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(json!({"status": 0, "error": "Character not found."}));
    };
    let current_name: String = row.try_get("name")?;
    if current_name.eq_ignore_ascii_case(&new_name) {
        return Ok(json!({"status": 2, "result": "Name is unchanged."}));
    }
    let taken: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM characters WHERE name=? AND id!=?")
        .bind(&new_name)
        .bind(char_id)
        .fetch_one(&mut *tx)
        .await?;
    if taken > 0 {
        return Ok(json!({"status": 2, "error": "Character name already taken."}));
    }
    let account_type: i8 = row.try_get("account_type")?;
    let required_qty = if account_type >= 1 { 1 } else { 3 };
    if !crate::inventory::take(&mut tx, char_id, "essential_01", required_qty).await? {
        return Ok(json!({
            "status": 2,
            "result": format!("You need {required_qty} Rename Badge(s) to rename your character.")
        }));
    }
    sqlx::query("UPDATE characters SET name=?, updated_at=NOW() WHERE id=?")
        .bind(&new_name)
        .bind(char_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(json!({"status": 1}))
}

pub async fn dev_add_items(pool: &MySqlPool, args: &[Json]) -> Result {
    let char_id = integer(args, 0);
    let session_key = string(args, 1);
    let item_string = string(args, 2);
    if let Some(error) = guard(pool, char_id, &session_key, true).await? {
        return Ok(error);
    }
    let mut tx = pool.begin().await?;
    let Some(account_type) = sqlx::query_scalar::<_, i8>(
        "SELECT u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE",
    )
    .bind(char_id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(json!({"status": 0, "error": "Character not found"}));
    };
    if account_type != 99 {
        return Ok(json!({"status": 0, "error": "Permission denied."}));
    }
    for part in item_string
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let (item_id, qty) = part
            .split_once(':')
            .map_or((part, 1), |(i, q)| (i, q.parse::<i64>().unwrap_or(1)));
        crate::rewards::give_item(&mut tx, char_id, item_id, qty.max(1)).await?;
    }
    tx.commit().await?;
    Ok(json!({
        "status": 1,
        "result": "Items added successfully!",
        "items": item_string
    }))
}

pub async fn dev_set_char_info(pool: &MySqlPool, args: &[Json]) -> Result {
    let char_id = integer(args, 0);
    let session_key = string(args, 1);
    let level = integer(args, 2);
    let rank_raw = integer(args, 3);
    if let Some(error) = guard(pool, char_id, &session_key, true).await? {
        return Ok(error);
    }
    let Some(account_type) = sqlx::query_scalar::<_, i8>(
        "SELECT u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=?",
    )
    .bind(char_id)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(json!({"status": 0, "error": "Character not found"}));
    };
    if account_type != 99 {
        return Ok(json!({"status": 0, "error": "Permission denied."}));
    }
    let final_rank = match rank_raw {
        1 => 1,
        2 => 3,
        3 => 5,
        4 => 7,
        r => r,
    };
    sqlx::query("UPDATE characters SET level=?, `rank`=?, updated_at=NOW() WHERE id=?")
        .bind(level)
        .bind(final_rank)
        .bind(char_id)
        .execute(pool)
        .await?;
    Ok(json!({"status": 1, "result": "Character updated!"}))
}

pub async fn dev_toggle_emblem(pool: &MySqlPool, args: &[Json]) -> Result {
    let char_id = integer(args, 0);
    let session_key = string(args, 1);
    if let Some(error) = guard(pool, char_id, &session_key, true).await? {
        return Ok(error);
    }
    let Some(row) = sqlx::query(
        "SELECT c.user_id, u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=?",
    )
    .bind(char_id)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(json!({"status": 0, "error": "Character not found"}));
    };
    let account_type: i8 = row.try_get("account_type")?;
    if account_type != 99 {
        return Ok(json!({"status": 0, "error": "Permission denied."}));
    }
    let user_id: u64 = row.try_get("user_id")?;
    let next_type = if account_type == 1 { 0 } else { 1 };
    sqlx::query("UPDATE users SET account_type=?, updated_at=NOW() WHERE id=?")
        .bind(next_type)
        .bind(user_id)
        .execute(pool)
        .await?;
    let status = if next_type == 1 { "Premium" } else { "Free" };
    Ok(json!({"status": 1, "result": format!("Account status: {status}")}))
}

pub async fn dev_learn_skill(pool: &MySqlPool, args: &[Json]) -> Result {
    let char_id = integer(args, 0);
    let session_key = string(args, 1);
    let skill_id = string(args, 2);
    if let Some(error) = guard(pool, char_id, &session_key, true).await? {
        return Ok(error);
    }
    let mut tx = pool.begin().await?;
    let Some(row) = sqlx::query(
        "SELECT c.level, c.element_1, c.element_2, c.element_3, u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE",
    )
    .bind(char_id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(json!({"status": 0, "error": "Character not found"}));
    };
    let account_type: i8 = row.try_get("account_type")?;
    if account_type != 99 {
        return Ok(json!({"status": 0, "error": "Permission denied."}));
    }
    let exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM character_skills WHERE character_id=? AND skill_id=?",
    )
    .bind(char_id)
    .bind(&skill_id)
    .fetch_one(&mut *tx)
    .await?;
    if exists > 0 {
        return Ok(json!({"status": 2}));
    }
    let Some(skill) = sqlx::query("SELECT level, element FROM skills WHERE skill_id=? LIMIT 1")
        .bind(&skill_id)
        .fetch_optional(&mut *tx)
        .await?
    else {
        return Ok(json!({"status": 0, "error": "Skill data not found!"}));
    };
    let char_lvl: i32 = row.try_get("level")?;
    let skill_lvl: i32 = skill.try_get("level")?;
    if char_lvl < skill_lvl {
        return Ok(json!({"status": 5}));
    }
    let mut e1: i32 = row.try_get("element_1")?;
    let mut e2: i32 = row.try_get("element_2")?;
    let mut e3: i32 = row.try_get("element_3")?;
    let element: i32 = skill.try_get("element")?;
    if (1..=5).contains(&element) && ![e1, e2, e3].contains(&element) {
        if e1 == 0 {
            e1 = element;
        } else if e2 == 0 {
            e2 = element;
        } else if e3 == 0 {
            e3 = element;
        } else {
            return Ok(json!({"status": 4}));
        }
        sqlx::query(
            "UPDATE characters SET element_1=?, element_2=?, element_3=?, updated_at=NOW() WHERE id=?",
        )
        .bind(e1)
        .bind(e2)
        .bind(e3)
        .bind(char_id)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query(
        "INSERT INTO character_skills (character_id, skill_id, created_at, updated_at) VALUES (?, ?, NOW(), NOW())",
    )
    .bind(char_id)
    .bind(&skill_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(json!({
        "status": 1,
        "result": "Skill learned.",
        "data": {
            "character_element_1": e1,
            "character_element_2": e2,
            "character_element_3": e3
        }
    }))
}
