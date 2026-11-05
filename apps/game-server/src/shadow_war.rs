use crate::GameState;
use crate::account::{guard, integer, session_error, string};
use crate::rewards;
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};
use std::time::{SystemTime, UNIX_EPOCH};

type Result = std::result::Result<Json, sqlx::Error>;

const MAX_ENERGY: i32 = 100;
const REFILL_COST: i32 = 50;
const REFILL_AMOUNT: i32 = 25;
const MAX_REFILLS_PER_DAY: i32 = 4;
const REFRESH_COST: i32 = 30;
const ENERGY_COST: i32 = 20;

#[derive(Clone, Debug)]
pub struct ShadowWarBattle {
    pub code: String,
    pub enemy_id: i64,
    pub season_id: u64,
    pub energy_before: i32,
    pub energy_after: i32,
    pub refills_today: i32,
}

pub async fn dispatch(state: &GameState, pool: &MySqlPool, method: &str, args: &[Json]) -> Result {
    match method {
        "getSeason" => get_season(pool, args).await,
        "getStatus" => get_status(pool, args).await,
        "getPresets" => get_presets(state, pool, args).await,
        "savePreset" => save_preset(state, pool, args).await,
        "usePreset" => use_preset(state, pool, args).await,
        "getEnemies" => get_enemies(state, pool, args).await,
        "refreshEnemies" => refresh_enemies(state, pool, args).await,
        "refillEnergy" => refill_energy(pool, args).await,
        "startBattle" => start_battle(state, pool, args).await,
        "finishBattle" => finish_battle(state, pool, args).await,
        "getEnemyInfo" => get_enemy_info(pool, args).await,
        "globalLeaderboard" => global_leaderboard(pool, args).await,
        "squadLeaderboard" => squad_leaderboard(pool, args).await,
        "getProfile" => get_profile(pool, args).await,
        _ => Ok(session_error(&format!(
            "Invalid ShadowWarService method: {method}"
        ))),
    }
}

pub async fn dispatch_clan_or_crew(pool: &MySqlPool, method: &str, args: &[Json]) -> Result {
    if method != "executeService"
        && let Some(error) = guard(pool, integer(args, 0), &string(args, 1), true).await?
    {
        return Ok(error);
    }
    match method {
        "executeService" => {
            let sub = string(args, 0);
            if let Some(params) = args.get(1).and_then(Json::as_array) {
                Box::pin(dispatch_clan_or_crew(pool, &sub, params)).await
            } else {
                Ok(not_in_crew())
            }
        }
        "getCharacterClan" | "getCharacterCrew" | "getClan" | "getCrew" | "getClanData"
        | "getCrewData" | "getClanInfo" | "getCrewInfo" | "getStatus" | "getData" | "init"
        | "login" | "loginToClanServer" | "loginToCrewServer" => {
            let id = integer(args, 0);
            let mut out = not_in_crew();
            let mut tx = pool.begin().await?;
            let user_id: u64 =
                sqlx::query_scalar("SELECT user_id FROM characters WHERE id=? FOR UPDATE")
                    .bind(id)
                    .fetch_one(&mut *tx)
                    .await?;
            for (table, field) in [
                ("clan_auth_tokens", "clan_token"),
                ("crew_auth_tokens", "crew_token"),
            ] {
                let first = authentication::random_hex()
                    .map_err(|_| sqlx::Error::Protocol("Entropy unavailable".into()))?;
                let second = authentication::random_hex()
                    .map_err(|_| sqlx::Error::Protocol("Entropy unavailable".into()))?;
                let token = format!("{first}{second}");
                sqlx::query(&format!("DELETE FROM {table} WHERE character_id=?"))
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
                sqlx::query(&format!("INSERT INTO {table} (user_id,character_id,token,expires_at,created_at,updated_at) VALUES (?,?,?,DATE_ADD(NOW(),INTERVAL 24 HOUR),NOW(),NOW())"))
                    .bind(user_id).bind(id).bind(&token).execute(&mut *tx).await?;
                out[field] = json!(token);
            }
            tx.commit().await?;
            Ok(out)
        }
        "getAllClans" | "getCrewList" | "getClanList" | "searchClan" | "searchCrew" => Ok(json!({
            "status": 1,
            "error": 0,
            "clans": [],
            "crews": [],
            "list": []
        })),
        "createClan" | "createCrew" | "joinClan" | "joinCrew" | "requestJoin" => Ok(json!({
            "status": 2,
            "error": 0,
            "result": "Clan/Crew creation and joining is not available right now."
        })),
        "leaveClan" | "leaveCrew" | "disbandClan" | "disbandCrew" => Ok(json!({
            "status": 1,
            "error": 0,
            "result": "You are not in a clan/crew."
        })),
        "kickMember" | "promoteMember" | "donate" | "donateGold" | "donateToken"
        | "donateTokens" => Ok(json!({
            "status": 2,
            "error": 0,
            "result": "You are not in a clan/crew."
        })),
        "getMembers" | "getClanMembers" | "getCrewMembers" => Ok(json!({
            "status": 1,
            "error": 0,
            "members": []
        })),
        "getLeaderboard" | "getClanLeaderboard" | "getCrewLeaderboard" => Ok(json!({
            "status": 1,
            "error": 0,
            "leaderboard": [],
            "list": []
        })),
        "getCrewBattleData" | "getClanBattleData" | "getBattleData" => Ok(json!({
            "status": 1,
            "error": 0,
            "battles": [],
            "data": null
        })),
        "startBattle" | "finishBattle" => Ok(json!({
            "status": 2,
            "error": 0,
            "result": "You are not in a clan/crew."
        })),
        "getMiniGameData" | "getMinigame" => Ok(json!({
            "status": 1,
            "error": 0,
            "data": null
        })),
        "getVillageData" | "getVillage" => Ok(json!({
            "status": 1,
            "error": 0,
            "village": null
        })),
        "upgradeVillage" | "upgradeBuilding" => Ok(json!({
            "status": 2,
            "error": 0,
            "result": "You are not in a clan/crew."
        })),
        _ => Ok(not_in_crew()),
    }
}

fn not_in_crew() -> Json {
    json!({
        "status": 1,
        "error": 0,
        "clan": null,
        "crew": null,
        "clan_id": 0,
        "crew_id": 0,
        "clan_name": "",
        "crew_name": "",
        "result": "not_in_crew"
    })
}

async fn check_access(pool: &MySqlPool, id: i64) -> std::result::Result<Option<Json>, sqlx::Error> {
    let Some(row) = sqlx::query("SELECT `rank`, level FROM characters WHERE id=?")
        .bind(id)
        .fetch_optional(pool)
        .await?
    else {
        return Ok(Some(json!({"status":0,"error":"Character not found"})));
    };
    let rank: i8 = row.try_get("rank")?;
    let level: i32 = row.try_get("level")?;
    if rank < 7 && level < 60 {
        return Ok(Some(json!({
            "status": 2,
            "result": "Shadow War requires Special Jounin rank (level 60) or higher."
        })));
    }
    Ok(None)
}

async fn get_active_season(
    pool: &MySqlPool,
) -> std::result::Result<Option<(u64, i32, i64)>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id, num, GREATEST(0, TIMESTAMPDIFF(SECOND, NOW(), end_at)) AS secs \
         FROM shadow_war_seasons WHERE active=1 ORDER BY num DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| {
        (
            r.try_get::<u64, _>("id").unwrap_or(1),
            r.try_get::<i32, _>("num").unwrap_or(1),
            r.try_get::<i64, _>("secs").unwrap_or(0),
        )
    }))
}

async fn get_or_create_squad(
    pool: &MySqlPool,
    id: i64,
    season_id: u64,
) -> std::result::Result<(u64, i32, i32, i32, i32, i32), sqlx::Error> {
    let today: String = sqlx::query_scalar("SELECT CAST(CURDATE() AS CHAR)")
        .fetch_one(pool)
        .await?;
    let existing = sqlx::query(
        "SELECT id, squad, `rank`, trophy, energy, CAST(energy_last_reset AS CHAR) AS last_reset, energy_refills_today \
         FROM shadow_war_squads WHERE character_id=? AND season_id=?",
    )
    .bind(id)
    .bind(season_id)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = existing {
        let sq_id: u64 = row.try_get("id")?;
        let squad: u8 = row.try_get("squad")?;
        let rank: i32 = row.try_get("rank")?;
        let trophy: i32 = row.try_get("trophy")?;
        let mut energy: i32 = row.try_get("energy")?;
        let last_reset: Option<String> = row.try_get("last_reset").ok().flatten();
        let mut refills: i32 = row.try_get("energy_refills_today")?;

        if last_reset.as_deref() != Some(today.as_str()) {
            energy = MAX_ENERGY;
            refills = 0;
            sqlx::query(
                "UPDATE shadow_war_squads SET energy=?, energy_last_reset=CURDATE(), energy_refills_today=0, updated_at=NOW() WHERE id=?",
            )
            .bind(energy)
            .bind(sq_id)
            .execute(pool)
            .await?;
        }
        return Ok((sq_id, squad as i32, rank, trophy, energy, refills));
    }

    let counts = sqlx::query(
        "SELECT squad, COUNT(*) AS cnt FROM shadow_war_squads WHERE season_id=? GROUP BY squad",
    )
    .bind(season_id)
    .fetch_all(pool)
    .await?;
    let mut squad_counts = [0i64; 5];
    for r in counts {
        let s: u8 = r.try_get("squad").unwrap_or(0);
        let c: i64 = r.try_get("cnt").unwrap_or(0);
        if (s as usize) < 5 {
            squad_counts[s as usize] = c;
        }
    }
    let min_squad = squad_counts
        .iter()
        .enumerate()
        .min_by_key(|(_, c)| *c)
        .map(|(idx, _)| idx as i32)
        .unwrap_or(0);

    let res = sqlx::query(
        "INSERT INTO shadow_war_squads (character_id,season_id,squad,`rank`,trophy,energy,energy_last_reset,energy_refills_today,created_at,updated_at) \
         VALUES (?,?,?,0,0,?,CURDATE(),0,NOW(),NOW())",
    )
    .bind(id)
    .bind(season_id)
    .bind(min_squad)
    .bind(MAX_ENERGY)
    .execute(pool)
    .await?;
    Ok((res.last_insert_id(), min_squad, 0, 0, MAX_ENERGY, 0))
}

async fn get_season(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    if let Some(err) = check_access(pool, id).await? {
        return Ok(err);
    }
    let Some((_, num, secs)) = get_active_season(pool).await? else {
        return Ok(json!({
            "status": 1,
            "error": 0,
            "active": false,
            "season": { "no": 0, "time": 0 }
        }));
    };
    Ok(json!({
        "status": 1,
        "error": 0,
        "active": true,
        "season": {
            "no": num,
            "time": secs
        }
    }))
}

async fn get_status(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    if let Some(err) = check_access(pool, id).await? {
        return Ok(err);
    }
    let Some((season_id, _, _)) = get_active_season(pool).await? else {
        return Ok(json!({
            "status": 1,
            "error": 0,
            "squad": 0,
            "rank": 0,
            "trophy": 0,
            "energy": MAX_ENERGY,
            "squads": [0, 0, 0, 0, 0]
        }));
    };

    let (_, squad, rank, trophy, energy, _) = get_or_create_squad(pool, id, season_id).await?;
    let totals = sqlx::query(
        "SELECT squad, CAST(SUM(trophy) AS SIGNED) AS total FROM shadow_war_squads WHERE season_id=? GROUP BY squad",
    )
    .bind(season_id)
    .fetch_all(pool)
    .await?;
    let mut squads = [0i64; 5];
    for r in totals {
        let s: u8 = r.try_get("squad").unwrap_or(0);
        let t: i64 = r.try_get("total").unwrap_or(0);
        if (s as usize) < 5 {
            squads[s as usize] = t;
        }
    }
    Ok(json!({
        "status": 1,
        "error": 0,
        "squad": squad,
        "rank": rank,
        "trophy": trophy,
        "energy": energy,
        "squads": squads
    }))
}

async fn ensure_presets(
    state: &GameState,
    pool: &MySqlPool,
    id: i64,
) -> std::result::Result<(i64, Vec<Json>), sqlx::Error> {
    let mut map = state.shadow_war_presets.lock().await;
    if let Some(existing) = map.get(&id) {
        return Ok(existing.clone());
    }
    let row = sqlx::query("SELECT * FROM characters WHERE id=?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    let mut default_preset = json!({
        "id": 1,
        "name": "Preset 1",
        "weapon": "wpn_01",
        "clothing": "set_01_0",
        "hair": "hair_01_0",
        "back_item": "back_01",
        "accessory": "accessory_01",
        "hair_color": "0|0",
        "skin_color": "0|0",
        "skills": "",
        "pet": 0,
        "talents": [],
        "senjutsu": [],
        "stats": {
            "wind": 0,
            "fire": 0,
            "thunder": 0,
            "water": 0,
            "earth": 0,
            "free": 0
        }
    });
    if let Some(c) = row {
        let gender: i8 = c.try_get("gender").unwrap_or(0);
        let suffix = if gender == 0 { "_0" } else { "_1" };
        let hair_raw: Option<String> = c.try_get("hair_style").ok().flatten();
        let hair = match hair_raw.as_deref() {
            Some(h) if !h.is_empty() && h.chars().all(|ch| ch.is_ascii_digit()) => {
                format!("hair_{:0>2}{suffix}", h)
            }
            Some(h) if !h.is_empty() => h.to_string(),
            _ => format!("hair_01{suffix}"),
        };
        let talents: Vec<String> = [
            c.try_get::<Option<String>, _>("talent_1").ok().flatten(),
            c.try_get::<Option<String>, _>("talent_2").ok().flatten(),
            c.try_get::<Option<String>, _>("talent_3").ok().flatten(),
        ]
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        .collect();
        let senjutsu: Vec<String> = c
            .try_get::<Option<String>, _>("senjutsu_equipped_skills")
            .ok()
            .flatten()
            .unwrap_or_default()
            .split(',')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        default_preset = json!({
            "id": 1,
            "name": "Preset 1",
            "weapon": c.try_get::<Option<String>, _>("equipment_weapon").ok().flatten().filter(|s| !s.is_empty()).unwrap_or_else(|| "wpn_01".into()),
            "clothing": c.try_get::<Option<String>, _>("equipment_clothing").ok().flatten().filter(|s| !s.is_empty()).unwrap_or_else(|| format!("set_01{suffix}")),
            "hair": hair,
            "back_item": c.try_get::<Option<String>, _>("equipment_back").ok().flatten().filter(|s| !s.is_empty()).unwrap_or_else(|| "back_01".into()),
            "accessory": c.try_get::<Option<String>, _>("equipment_accessory").ok().flatten().filter(|s| !s.is_empty()).unwrap_or_else(|| "accessory_01".into()),
            "hair_color": c.try_get::<Option<String>, _>("hair_color").ok().flatten().filter(|s| !s.is_empty()).unwrap_or_else(|| "0|0".into()),
            "skin_color": c.try_get::<Option<String>, _>("skin_color").ok().flatten().filter(|s| !s.is_empty()).unwrap_or_else(|| "0|0".into()),
            "skills": c.try_get::<Option<String>, _>("equipment_skills").ok().flatten().unwrap_or_default(),
            "pet": c.try_get::<Option<String>, _>("equipment_pet").ok().flatten().filter(|s| !s.is_empty()).map(|s| json!(s)).unwrap_or_else(|| json!(0)),
            "talents": talents,
            "senjutsu": senjutsu,
            "stats": {
                "wind": c.try_get::<i32, _>("point_wind").unwrap_or(0),
                "fire": c.try_get::<i32, _>("point_fire").unwrap_or(0),
                "thunder": c.try_get::<i32, _>("point_lightning").unwrap_or(0),
                "water": c.try_get::<i32, _>("point_water").unwrap_or(0),
                "earth": c.try_get::<i32, _>("point_earth").unwrap_or(0),
                "free": c.try_get::<i32, _>("point_free").unwrap_or(0)
            }
        });
    }
    let mut p2 = default_preset.clone();
    p2["id"] = json!(2);
    p2["name"] = json!("Preset 2");
    let mut p3 = default_preset.clone();
    p3["id"] = json!(3);
    p3["name"] = json!("Preset 3");
    let list = vec![default_preset, p2, p3];
    map.insert(id, (1, list.clone()));
    Ok((1, list))
}

async fn get_presets(state: &GameState, pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    if let Some(err) = check_access(pool, id).await? {
        return Ok(err);
    }
    let (active_id, presets) = ensure_presets(state, pool, id).await?;
    Ok(json!({
        "status": 1,
        "error": 0,
        "active": active_id,
        "presets": presets
    }))
}

async fn save_preset(state: &GameState, pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let preset_data = args.get(2).cloned().unwrap_or_else(|| json!({}));
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let (active_id, mut presets) = ensure_presets(state, pool, id).await?;
    let preset_id = preset_data.get("id").and_then(Json::as_i64).unwrap_or(1);
    let mut found = false;
    for p in &mut presets {
        if p.get("id").and_then(Json::as_i64) == Some(preset_id) {
            if let (Some(dst), Some(src)) = (p.as_object_mut(), preset_data.as_object()) {
                for (k, v) in src {
                    dst.insert(k.clone(), v.clone());
                }
                dst.insert("id".into(), json!(preset_id));
            }
            found = true;
            break;
        }
    }
    if !found {
        let mut new_p = preset_data;
        if let Some(obj) = new_p.as_object_mut() {
            obj.insert("id".into(), json!(preset_id));
        }
        presets.push(new_p);
    }
    state
        .shadow_war_presets
        .lock()
        .await
        .insert(id, (active_id, presets.clone()));
    Ok(json!({
        "status": 1,
        "error": 0,
        "active": active_id,
        "presets": presets,
        "result": "Preset saved."
    }))
}

async fn use_preset(state: &GameState, pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let preset_id = integer(args, 2).max(1);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let (_, presets) = ensure_presets(state, pool, id).await?;
    state
        .shadow_war_presets
        .lock()
        .await
        .insert(id, (preset_id, presets.clone()));
    Ok(json!({
        "status": 1,
        "error": 0,
        "active": preset_id,
        "presets": presets,
        "result": "Defender preset updated."
    }))
}

async fn build_enemy_list(
    pool: &MySqlPool,
    id: i64,
) -> std::result::Result<Vec<Json>, sqlx::Error> {
    let season_opt = get_active_season(pool).await?;
    let season_id = season_opt.map(|(sid, _, _)| sid).unwrap_or(0);
    let mut rows = sqlx::query(
        "SELECT c.*, u.account_type FROM characters c JOIN users u ON u.id=c.user_id \
         WHERE c.id<>? AND (c.rank>=7 OR c.level>=60) ORDER BY RAND() LIMIT 6",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    if rows.is_empty() {
        rows = sqlx::query(
            "SELECT c.*, u.account_type FROM characters c JOIN users u ON u.id=c.user_id \
             WHERE c.id<>? ORDER BY RAND() LIMIT 6",
        )
        .bind(id)
        .fetch_all(pool)
        .await?;
    }

    let mut result = Vec::new();
    for c in rows {
        let cid: u64 = c.try_get("id")?;
        let mut squad_num = 0i32;
        let mut rank_num = 0i32;
        let mut trophy_num = 0i32;
        if season_id > 0
            && let Some(sq) = sqlx::query(
                "SELECT squad, `rank`, trophy FROM shadow_war_squads WHERE character_id=? AND season_id=?",
            )
            .bind(cid)
            .bind(season_id)
            .fetch_optional(pool)
            .await?
        {
            squad_num = sq.try_get::<u8, _>("squad").unwrap_or(0) as i32;
            rank_num = sq.try_get::<i32, _>("rank").unwrap_or(0);
            trophy_num = sq.try_get::<i32, _>("trophy").unwrap_or(0);
        }

        let gender: i8 = c.try_get("gender").unwrap_or(0);
        let suffix = if gender == 0 { "_0" } else { "_1" };
        let hair_raw: Option<String> = c.try_get("hair_style").ok().flatten();
        let hair = match hair_raw.as_deref() {
            Some(h) if !h.is_empty() && h.chars().all(|ch| ch.is_ascii_digit()) => {
                format!("hair_{:0>2}{suffix}", h)
            }
            Some(h) if !h.is_empty() => h.to_string(),
            _ => format!("hair_01{suffix}"),
        };
        let weapon = c
            .try_get::<Option<String>, _>("equipment_weapon")?
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "wpn_01".into());
        let back_item = c
            .try_get::<Option<String>, _>("equipment_back")?
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "back_01".into());
        let clothing = c
            .try_get::<Option<String>, _>("equipment_clothing")?
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("set_01{suffix}"));
        let accessory = c
            .try_get::<Option<String>, _>("equipment_accessory")?
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "accessory_01".into());
        let hair_color = c
            .try_get::<Option<String>, _>("hair_color")?
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "0|0".into());
        let skin_color = c
            .try_get::<Option<String>, _>("skin_color")?
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "0|0".into());
        let pet_val = match c
            .try_get::<Option<String>, _>("equipment_pet")?
            .filter(|s| !s.is_empty())
        {
            Some(p) => json!(p),
            None => json!(0),
        };

        result.push(json!({
            "id": cid,
            "char_id": cid,
            "name": c.try_get::<String, _>("name")?,
            "level": c.try_get::<i32, _>("level")?,
            "rank": c.try_get::<i8, _>("rank")?,
            "gender": gender,
            "squad": squad_num,
            "sw_rank": rank_num,
            "trophy": trophy_num,
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
        }));
    }
    Ok(result)
}

async fn get_enemies(state: &GameState, pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    if let Some(err) = check_access(pool, id).await? {
        return Ok(err);
    }
    {
        let map = state.shadow_war_enemies.lock().await;
        if let Some(cached) = map.get(&id) {
            return Ok(json!({
                "status": 1,
                "error": 0,
                "enemies": cached
            }));
        }
    }
    let enemies = build_enemy_list(pool, id).await?;
    state
        .shadow_war_enemies
        .lock()
        .await
        .insert(id, enemies.clone());
    Ok(json!({
        "status": 1,
        "error": 0,
        "enemies": enemies
    }))
}

async fn refresh_enemies(state: &GameState, pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let mut tx = pool.begin().await?;
    let Some(user_row) = sqlx::query(
        "SELECT u.id, u.tokens FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(json!({"status":0,"error":"User not found"}));
    };
    let uid: u64 = user_row.try_get("id")?;
    let tokens: i32 = user_row.try_get("tokens")?;
    if tokens < REFRESH_COST {
        return Ok(json!({"status":2,"result":"Not enough tokens! (30 tokens required)"}));
    }
    sqlx::query("UPDATE users SET tokens=tokens-?, updated_at=NOW() WHERE id=?")
        .bind(REFRESH_COST)
        .bind(uid)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let enemies = build_enemy_list(pool, id).await?;
    state
        .shadow_war_enemies
        .lock()
        .await
        .insert(id, enemies.clone());
    Ok(json!({
        "status": 1,
        "error": 0,
        "enemies": enemies
    }))
}

async fn refill_energy(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let Some((season_id, _, _)) = get_active_season(pool).await? else {
        return Ok(json!({"status":2,"result":"No active Shadow War season."}));
    };
    let (sq_id, _, _, _, energy, refills) = get_or_create_squad(pool, id, season_id).await?;
    if refills >= MAX_REFILLS_PER_DAY {
        return Ok(json!({
            "status": 2,
            "result": "You have reached the daily energy refill limit."
        }));
    }
    let mut tx = pool.begin().await?;
    let Some(user_row) = sqlx::query(
        "SELECT u.id, u.tokens FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=? FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(json!({"status":0,"error":"User not found"}));
    };
    let uid: u64 = user_row.try_get("id")?;
    let tokens: i32 = user_row.try_get("tokens")?;
    if tokens < REFILL_COST {
        return Ok(json!({"status":2,"result":"Not enough tokens! (50 tokens required)"}));
    }
    let next_energy = (energy + REFILL_AMOUNT).min(MAX_ENERGY);
    sqlx::query("UPDATE users SET tokens=tokens-?, updated_at=NOW() WHERE id=?")
        .bind(REFILL_COST)
        .bind(uid)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE shadow_war_squads SET energy=?, energy_refills_today=energy_refills_today+1, updated_at=NOW() WHERE id=?",
    )
    .bind(next_energy)
    .bind(sq_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(json!({
        "status": 1,
        "error": 0,
        "energy": next_energy
    }))
}

async fn start_battle(state: &GameState, pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let enemy_id = integer(args, 2);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    if let Some(err) = check_access(pool, id).await? {
        return Ok(err);
    }
    let Some((season_id, _, _)) = get_active_season(pool).await? else {
        return Ok(json!({"status":2,"result":"No active Shadow War season."}));
    };
    let (sq_id, _, _, _, energy, refills) = get_or_create_squad(pool, id, season_id).await?;
    if energy < ENERGY_COST {
        return Ok(json!({"status":2,"result":"Not enough energy! (20 required)"}));
    }
    let energy_after = energy - ENERGY_COST;
    sqlx::query("UPDATE shadow_war_squads SET energy=?, updated_at=NOW() WHERE id=?")
        .bind(energy_after)
        .bind(sq_id)
        .execute(pool)
        .await?;

    let battle_code = authentication::random_hex()
        .unwrap_or_else(|_| "0123456789abcdef".into())
        .chars()
        .take(12)
        .collect::<String>();
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let hash = authentication::sha256_hex(&format!("{battle_code}{id}{enemy_id}{now_secs}"));

    state.shadow_war_battles.lock().await.insert(
        id,
        ShadowWarBattle {
            code: battle_code.clone(),
            enemy_id,
            season_id,
            energy_before: energy,
            energy_after,
            refills_today: refills,
        },
    );

    Ok(json!({
        "status": 1,
        "error": 0,
        "id": battle_code,
        "energy": energy_after,
        "hash": hash
    }))
}

fn calculate_rank(trophy: i32) -> i32 {
    if trophy >= 2500 {
        7
    } else if trophy >= 1800 {
        6
    } else if trophy >= 1200 {
        5
    } else if trophy >= 700 {
        4
    } else if trophy >= 350 {
        3
    } else if trophy >= 150 {
        2
    } else if trophy >= 50 {
        1
    } else {
        0
    }
}

async fn finish_battle(state: &GameState, pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let battle_code = string(args, 2);
    let total_damage = integer(args, 3) as i32;
    let battle_data_b64 = string(args, 4);
    let won = integer(args, 6) == 1 || args.get(6).and_then(Json::as_bool).unwrap_or(false);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }

    let session = {
        let mut map = state.shadow_war_battles.lock().await;
        match map.get(&id) {
            Some(s) if s.code == battle_code => map.remove(&id).unwrap(),
            _ => return Ok(json!({"status":2,"result":"Invalid or expired battle session."})),
        }
    };

    let Some(char_row) = sqlx::query("SELECT level, `rank` FROM characters WHERE id=?")
        .bind(id)
        .fetch_optional(pool)
        .await?
    else {
        return Ok(json!({"status":0,"error":"Character not found"}));
    };
    let char_level: i32 = char_row.try_get("level")?;
    let char_rank: i8 = char_row.try_get("rank")?;

    let (sq_id, char_squad, _, trophy_before, _, _) =
        get_or_create_squad(pool, id, session.season_id).await?;
    let trophy_delta = if won {
        15 + (total_damage / 500).clamp(0, 15)
    } else {
        -10
    };
    let trophy_after = (trophy_before + trophy_delta).max(0);
    let next_rank = calculate_rank(trophy_after);

    sqlx::query("UPDATE shadow_war_squads SET trophy=?, `rank`=?, updated_at=NOW() WHERE id=?")
        .bind(trophy_after)
        .bind(next_rank)
        .bind(sq_id)
        .execute(pool)
        .await?;

    let enemy_row = sqlx::query("SELECT id, level, `rank` FROM characters WHERE id=?")
        .bind(session.enemy_id)
        .fetch_optional(pool)
        .await?;
    let mut enemy_trophy_before = 0;
    let mut enemy_trophy_after = 0;
    let mut enemy_squad: Option<i32> = None;
    let mut enemy_level = 0;
    let mut enemy_rank: i8 = 0;
    if let Some(er) = &enemy_row {
        enemy_level = er.try_get("level").unwrap_or(0);
        enemy_rank = er.try_get("rank").unwrap_or(0);
        if let Some(esq) = sqlx::query(
            "SELECT id, squad, trophy FROM shadow_war_squads WHERE character_id=? AND season_id=?",
        )
        .bind(session.enemy_id)
        .bind(session.season_id)
        .fetch_optional(pool)
        .await?
        {
            let esq_id: u64 = esq.try_get("id")?;
            let esq_squad: u8 = esq.try_get("squad")?;
            let et_before: i32 = esq.try_get("trophy")?;
            enemy_squad = Some(esq_squad as i32);
            enemy_trophy_before = et_before;
            enemy_trophy_after = if won {
                (et_before - 8).max(0)
            } else {
                et_before + 5
            };
            let erank = calculate_rank(enemy_trophy_after);
            sqlx::query(
                "UPDATE shadow_war_squads SET trophy=?, `rank`=?, updated_at=NOW() WHERE id=?",
            )
            .bind(enemy_trophy_after)
            .bind(erank)
            .bind(esq_id)
            .execute(pool)
            .await?;
        }
    }

    let battle_json = json!({"raw_b64": battle_data_b64}).to_string();
    sqlx::query(
        "INSERT INTO shadow_war_battles \
         (season_id,battle_code,character_id,enemy_id,character_squad,enemy_squad,character_level,enemy_level,character_rank,enemy_rank,character_trophy_before,enemy_trophy_before,character_trophy_after,enemy_trophy_after,trophy_delta,total_damage,won,battle_data,energy_cost,energy_before,energy_after,refills_used_today,created_at,updated_at) \
         VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,NOW(),NOW())",
    )
    .bind(session.season_id)
    .bind(&battle_code)
    .bind(id)
    .bind(if enemy_row.is_some() { Some(session.enemy_id) } else { None })
    .bind(char_squad)
    .bind(enemy_squad)
    .bind(char_level)
    .bind(enemy_level)
    .bind(char_rank)
    .bind(enemy_rank)
    .bind(trophy_before)
    .bind(enemy_trophy_before)
    .bind(trophy_after)
    .bind(enemy_trophy_after)
    .bind(trophy_delta)
    .bind(total_damage)
    .bind(won)
    .bind(battle_json)
    .bind(ENERGY_COST)
    .bind(session.energy_before)
    .bind(session.energy_after)
    .bind(session.refills_today)
    .execute(pool)
    .await?;

    let reward_gold = if won {
        i64::from((char_level * 50).max(500))
    } else {
        i64::from((char_level * 15).max(100))
    };
    let reward_xp = if won {
        i64::from((char_level * 80).max(500))
    } else {
        i64::from((char_level * 20).max(100))
    };

    let mut tx = pool.begin().await?;
    rewards::add_gold(&mut tx, id, reward_gold).await?;
    let (level, xp, level_up) = rewards::add_xp(&mut tx, id, reward_xp).await?;
    tx.commit().await?;

    state.shadow_war_enemies.lock().await.remove(&id);

    Ok(json!({
        "status": 1,
        "error": 0,
        "result": [reward_gold, reward_xp, []],
        "level": level,
        "xp": xp,
        "level_up": level_up,
        "trophy": trophy_after,
        "trophy_delta": trophy_delta,
        "rank": next_rank
    }))
}

async fn get_enemy_info(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let target_id = integer(args, 2);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let Some(enemy) = sqlx::query(
        "SELECT c.*, u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=?",
    )
    .bind(target_id)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(json!({"status":2,"result":"Enemy character not found."}));
    };

    let gender: i8 = enemy.try_get("gender").unwrap_or(0);
    let suffix = if gender == 0 { "_0" } else { "_1" };
    let hair_raw: Option<String> = enemy.try_get("hair_style").ok().flatten();
    let hair = match hair_raw.as_deref() {
        Some(h) if !h.is_empty() && h.chars().all(|ch| ch.is_ascii_digit()) => {
            format!("hair_{:0>2}{suffix}", h)
        }
        Some(h) if !h.is_empty() => h.to_string(),
        _ => format!("hair_01{suffix}"),
    };
    let weapon = enemy
        .try_get::<Option<String>, _>("equipment_weapon")?
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "wpn_01".into());
    let back_item = enemy
        .try_get::<Option<String>, _>("equipment_back")?
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "back_01".into());
    let clothing = enemy
        .try_get::<Option<String>, _>("equipment_clothing")?
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("set_01{suffix}"));
    let accessory = enemy
        .try_get::<Option<String>, _>("equipment_accessory")?
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "accessory_01".into());
    let hair_color = enemy
        .try_get::<Option<String>, _>("hair_color")?
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "0|0".into());
    let skin_color = enemy
        .try_get::<Option<String>, _>("skin_color")?
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "0|0".into());
    let skills = enemy
        .try_get::<Option<String>, _>("equipment_skills")?
        .unwrap_or_default();
    let senjutsu_skills = enemy
        .try_get::<Option<String>, _>("senjutsu_equipped_skills")?
        .unwrap_or_default();
    let talent_skills = enemy
        .try_get::<Option<String>, _>("talent_skills")?
        .unwrap_or_default();
    let pet_val = match enemy
        .try_get::<Option<String>, _>("equipment_pet")?
        .filter(|s| !s.is_empty())
    {
        Some(p) => json!(p),
        None => json!(0),
    };

    let mut pet_data = json!({});
    if let Some(pet_ref) = enemy
        .try_get::<Option<String>, _>("equipment_pet")?
        .filter(|s| !s.is_empty() && s != "0")
    {
        let pet_row = if pet_ref.chars().all(|c| c.is_ascii_digit()) {
            sqlx::query("SELECT * FROM character_pets WHERE character_id=? AND id=?")
                .bind(target_id)
                .bind(pet_ref.parse::<i64>().unwrap_or(0))
                .fetch_optional(pool)
                .await?
        } else {
            sqlx::query(
                "SELECT * FROM character_pets WHERE character_id=? AND pet_id=? ORDER BY id LIMIT 1",
            )
            .bind(target_id)
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

    let eid: u64 = enemy.try_get("id")?;
    let ename: String = enemy.try_get("name")?;
    let elevel: i32 = enemy.try_get("level")?;
    let erank: i8 = enemy.try_get("rank")?;

    Ok(json!({
        "status": 1,
        "error": 0,
        "enemy": {
            "character_data": {
                "character_id": eid,
                "character_name": ename,
                "character_level": elevel,
                "character_xp": enemy.try_get::<i64, _>("xp").unwrap_or(0),
                "character_gender": gender,
                "character_rank": erank,
                "character_merit": 0,
                "character_prestige": enemy.try_get::<i32, _>("prestige").unwrap_or(0),
                "character_element_1": enemy.try_get::<i8, _>("element_1").unwrap_or(0),
                "character_element_2": enemy.try_get::<i8, _>("element_2").unwrap_or(0),
                "character_element_3": enemy.try_get::<i8, _>("element_3").unwrap_or(0),
                "character_talent_1": enemy.try_get::<Option<String>, _>("talent_1")?,
                "character_talent_2": enemy.try_get::<Option<String>, _>("talent_2")?,
                "character_talent_3": enemy.try_get::<Option<String>, _>("talent_3")?,
                "character_gold": enemy.try_get::<i32, _>("gold").unwrap_or(0),
                "character_tp": enemy.try_get::<i32, _>("tp").unwrap_or(0),
                "character_ss": enemy.try_get::<i32, _>("ss").unwrap_or(0),
                "character_class": enemy.try_get::<Option<String>, _>("class")?,
                "character_senjutsu": enemy.try_get::<Option<String>, _>("senjutsu_type")?,
                "character_pvp_points": enemy.try_get::<i32, _>("pvp_points").unwrap_or(0)
            },
            "character_points": {
                "atrrib_wind": enemy.try_get::<i32, _>("point_wind").unwrap_or(0),
                "atrrib_fire": enemy.try_get::<i32, _>("point_fire").unwrap_or(0),
                "atrrib_lightning": enemy.try_get::<i32, _>("point_lightning").unwrap_or(0),
                "atrrib_water": enemy.try_get::<i32, _>("point_water").unwrap_or(0),
                "atrrib_earth": enemy.try_get::<i32, _>("point_earth").unwrap_or(0),
                "atrrib_free": enemy.try_get::<i32, _>("point_free").unwrap_or(0)
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
                "talent_skills": talent_skills,
                "pet": pet_val
            },
            "pet_data": pet_data,
            "clan": null
        }
    }))
}

fn format_leaderboard_entry(r: &sqlx::mysql::MySqlRow, idx: usize) -> Json {
    let cid: u64 = r.try_get("character_id").unwrap_or(0);
    let name: String = r.try_get("name").unwrap_or_default();
    let level: i32 = r.try_get("level").unwrap_or(1);
    let rank: i8 = r.try_get("char_rank").unwrap_or(1);
    let gender: i8 = r.try_get("gender").unwrap_or(0);
    let suffix = if gender == 0 { "_0" } else { "_1" };
    let hair_raw: Option<String> = r.try_get("hair_style").ok().flatten();
    let hair = match hair_raw.as_deref() {
        Some(h) if !h.is_empty() && h.chars().all(|ch| ch.is_ascii_digit()) => {
            format!("hair_{:0>2}{suffix}", h)
        }
        Some(h) if !h.is_empty() => h.to_string(),
        _ => format!("hair_01{suffix}"),
    };
    let weapon = r
        .try_get::<Option<String>, _>("equipment_weapon")
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "wpn_01".into());
    let back_item = r
        .try_get::<Option<String>, _>("equipment_back")
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "back_01".into());
    let clothing = r
        .try_get::<Option<String>, _>("equipment_clothing")
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("set_01{suffix}"));
    let accessory = r
        .try_get::<Option<String>, _>("equipment_accessory")
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "accessory_01".into());
    let hair_color = r
        .try_get::<Option<String>, _>("hair_color")
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "0|0".into());
    let skin_color = r
        .try_get::<Option<String>, _>("skin_color")
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "0|0".into());
    json!({
        "rank": idx + 1,
        "id": cid,
        "char_id": cid,
        "name": name,
        "level": level,
        "char_rank": rank,
        "squad": r.try_get::<u8, _>("squad").unwrap_or(0),
        "sw_rank": r.try_get::<i32, _>("sw_rank").unwrap_or(0),
        "trophy": r.try_get::<i32, _>("trophy").unwrap_or(0),
        "sets": {
            "weapon": weapon,
            "back_item": back_item,
            "clothing": clothing,
            "hairstyle": hair,
            "accessory": accessory,
            "face": format!("face_01{suffix}"),
            "hair_color": hair_color,
            "skin_color": skin_color
        }
    })
}

async fn global_leaderboard(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let Some((season_id, _, _)) = get_active_season(pool).await? else {
        return Ok(json!({"status":1,"error":0,"leaderboard":[]}));
    };
    let rows = sqlx::query(
        "SELECT s.character_id, s.squad, s.rank AS sw_rank, s.trophy, \
         c.name, c.level, c.rank AS char_rank, c.gender, c.hair_style, \
         c.equipment_weapon, c.equipment_back, c.equipment_clothing, c.equipment_accessory, c.hair_color, c.skin_color \
         FROM shadow_war_squads s JOIN characters c ON c.id=s.character_id \
         WHERE s.season_id=? ORDER BY s.trophy DESC, s.id ASC LIMIT 50",
    )
    .bind(season_id)
    .fetch_all(pool)
    .await?;
    let list: Vec<Json> = rows
        .iter()
        .enumerate()
        .map(|(idx, r)| format_leaderboard_entry(r, idx))
        .collect();
    Ok(json!({
        "status": 1,
        "error": 0,
        "leaderboard": list
    }))
}

async fn squad_leaderboard(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let squad = integer(args, 2);
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let Some((season_id, _, _)) = get_active_season(pool).await? else {
        return Ok(json!({"status":1,"error":0,"leaderboard":[]}));
    };
    let rows = sqlx::query(
        "SELECT s.character_id, s.squad, s.rank AS sw_rank, s.trophy, \
         c.name, c.level, c.rank AS char_rank, c.gender, c.hair_style, \
         c.equipment_weapon, c.equipment_back, c.equipment_clothing, c.equipment_accessory, c.hair_color, c.skin_color \
         FROM shadow_war_squads s JOIN characters c ON c.id=s.character_id \
         WHERE s.season_id=? AND s.squad=? ORDER BY s.trophy DESC, s.id ASC LIMIT 50",
    )
    .bind(season_id)
    .bind(squad)
    .fetch_all(pool)
    .await?;
    let list: Vec<Json> = rows
        .iter()
        .enumerate()
        .map(|(idx, r)| format_leaderboard_entry(r, idx))
        .collect();
    Ok(json!({
        "status": 1,
        "error": 0,
        "leaderboard": list
    }))
}

async fn get_profile(pool: &MySqlPool, args: &[Json]) -> Result {
    let id = integer(args, 0);
    let key = string(args, 1);
    let target_id = integer(args, 2);
    let lookup_id = if target_id > 0 { target_id } else { id };
    if let Some(e) = guard(pool, id, &key, true).await? {
        return Ok(e);
    }
    let Some(c) = sqlx::query("SELECT id, name, level, `rank` FROM characters WHERE id=?")
        .bind(lookup_id)
        .fetch_optional(pool)
        .await?
    else {
        return Ok(json!({"status":2,"result":"Character not found."}));
    };
    let mut squad_data = json!({
        "squad": 0,
        "rank": 0,
        "trophy": 0
    });
    if let Some((season_id, _, _)) = get_active_season(pool).await?
        && let Some(sq) = sqlx::query(
            "SELECT squad, `rank`, trophy FROM shadow_war_squads WHERE character_id=? AND season_id=?",
        )
        .bind(lookup_id)
        .bind(season_id)
        .fetch_optional(pool)
        .await?
    {
        squad_data = json!({
            "squad": sq.try_get::<u8, _>("squad").unwrap_or(0),
            "rank": sq.try_get::<i32, _>("rank").unwrap_or(0),
            "trophy": sq.try_get::<i32, _>("trophy").unwrap_or(0)
        });
    }
    Ok(json!({
        "status": 1,
        "error": 0,
        "profile": {
            "id": c.try_get::<u64, _>("id")?,
            "name": c.try_get::<String, _>("name")?,
            "level": c.try_get::<i32, _>("level")?,
            "char_rank": c.try_get::<i8, _>("rank")?,
            "squad": squad_data["squad"],
            "sw_rank": squad_data["rank"],
            "trophy": squad_data["trophy"]
        }
    }))
}
