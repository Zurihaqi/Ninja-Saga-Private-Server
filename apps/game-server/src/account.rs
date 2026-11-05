use crate::{GameState, characters};
use amf_codec::Value;
use authentication as auth;
use persistence::{MySqlPool, Row, sqlx};
use serde_json::{Value as Json, json};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn arguments(value: &Value) -> Vec<Json> {
    let data = from_amf(value);
    let mut args = match data {
        Json::Array(args) => args,
        other => vec![other],
    };
    if args.len() == 1 && args[0].is_array() {
        args = args.remove(0).as_array().unwrap().clone();
    }
    args
}
fn from_amf(value: &Value) -> Json {
    match value {
        Value::Amf3(v) => from_amf(v),
        Value::Null | Value::Undefined => Json::Null,
        Value::Bool(v) => json!(v),
        Value::Number(v) => json!(v),
        Value::Integer(v) => json!(v),
        Value::String(v) => json!(v),
        Value::ByteArray(_) => Json::Null,
        Value::Array(v) => Json::Array(v.iter().map(from_amf).collect()),
        Value::Object(v) => Json::Object(v.iter().map(|(k, v)| (k.clone(), from_amf(v))).collect()),
        // ECMA arrays decode like PHP arrays: purely numeric keys become a
        // list, mixed keys stay associative (AIR clients wrap arguments this
        // way).
        Value::MixedArray(v) => {
            let mut numbers = Vec::with_capacity(v.len());
            for (key, _) in v.iter() {
                match key.parse::<usize>() {
                    Ok(n) => numbers.push(n),
                    Err(_) => {
                        return Json::Object(
                            v.iter().map(|(k, v)| (k.clone(), from_amf(v))).collect(),
                        );
                    }
                }
            }
            let mut sorted = numbers.clone();
            sorted.sort_unstable();
            if sorted.as_slice() == (0..numbers.len()).collect::<Vec<_>>().as_slice() {
                let mut dense = vec![Json::Null; numbers.len()];
                for (key, value) in v.iter() {
                    dense[key.parse::<usize>().unwrap()] = from_amf(value);
                }
                Json::Array(dense)
            } else {
                Json::Object(v.iter().map(|(k, v)| (k.clone(), from_amf(v))).collect())
            }
        }
    }
}
pub fn to_amf(value: &Json, amf3: bool) -> Value {
    match value {
        Json::Null => Value::Null,
        Json::Bool(v) => Value::Bool(*v),
        Json::String(v) => Value::String(v.clone()),
        Json::Number(v) => {
            if amf3
                && v.as_i64()
                    .is_some_and(|n| (-268435456..=268435455).contains(&n))
            {
                Value::Integer(v.as_i64().unwrap() as i32)
            } else {
                Value::Number(v.as_f64().unwrap_or(0.0))
            }
        }
        Json::Array(v) => Value::Array(v.iter().map(|v| to_amf(v, amf3)).collect()),
        Json::Object(v) => {
            let pairs = v
                .iter()
                .map(|(k, v)| (k.clone(), to_amf(v, amf3)))
                .collect();
            if amf3 {
                Value::Object(pairs)
            } else {
                Value::MixedArray(pairs)
            }
        }
    }
}
pub fn string(args: &[Json], index: usize) -> String {
    match args.get(index) {
        Some(Json::String(v)) => v.clone(),
        // PHP prints whole floats without a decimal point; Ruffle sends some
        // values as numbers that AIR sent as strings.
        Some(Json::Number(v)) => match v.as_f64() {
            Some(f) if f.is_finite() && f == f.trunc() => (f as i64).to_string(),
            _ => v.to_string(),
        },
        _ => String::new(),
    }
}
pub fn integer(args: &[Json], index: usize) -> i64 {
    args.get(index)
        .and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_f64().filter(|f| f.is_finite()).map(|f| f as i64))
                .or_else(|| v.as_str()?.parse().ok())
        })
        .unwrap_or(0)
}
pub fn session_error(message: &str) -> Json {
    json!({"status":0,"error":message,"errorMessage":message})
}

pub async fn dispatch(state: &GameState, target: &str, args: &[Json]) -> Json {
    let target = target.replace("Service.", ".");
    if target == "SystemLogin.checkVersion" {
        let random = match auth::random_hex() {
            Ok(v) => v,
            Err(_) => return json!({"status":500,"error":"Entropy unavailable"}),
        };
        let iv = 100000 + u32::from_str_radix(&random[..6], 16).unwrap_or(0) % 900000;
        let cdn = std::env::var("APP_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".into());
        return json!({"status":1,"error":0,"_":iv,"__":&random[..16],"cdn":format!("{}/",cdn.trim_end_matches('/')),"pvp_socket":std::env::var("PVP_SOCKET_URL").unwrap_or_else(|_|"http://127.0.0.1:3000/pvp".into()),"chat_socket":std::env::var("CHAT_SOCKET_URL").unwrap_or_else(|_|"http://127.0.0.1:3002".into()),"_rm":""});
    }
    let Some(pool) = &state.database else {
        return json!({"status":500,"error":"Database is not configured"});
    };
    let result = match target.as_str() {
        "Billing.claimProduct" => crate::billing::claim(pool, args).await,
        "Analytics.libraries" => Ok(json!({"status":1})),
        target if target.starts_with("ScrollOfWisdom.") => {
            crate::wisdom::dispatch(pool, target.split_once('.').unwrap().1, args).await
        }
        "SystemLogin.registerUser" => register(pool, args).await,
        "SystemLogin.loginUser" => login(pool, args).await,
        "SystemLogin.getAllCharacters" => characters::list(pool, args).await,
        "SystemLogin.getCharacterData" => characters::load(pool, args).await,
        "Character.characterRegister" => {
            characters::create(
                pool,
                if args.len() == 1 {
                    args[0].as_array().map(Vec::as_slice).unwrap_or(args)
                } else {
                    args
                },
            )
            .await
        }
        "Character.equipSet" => characters::equip(pool, args).await,
        "Character.equipSkillSet" => characters::equip_skills(pool, args).await,
        "Character.setPoints" => characters::set_points(pool, args).await,
        "Character.buyItem" => crate::economy::buy_item(pool, args).await,
        "Character.buyAnimation" => crate::animations::dispatch(pool, args, true).await,
        "Character.useAnimation" => crate::animations::dispatch(pool, args, false).await,
        "Character.buyRenameBadge" => {
            crate::economy::buy_essential(pool, args, "essential_01", "essential").await
        }
        "Character.buyTalentEssential" => {
            crate::economy::buy_essential(pool, args, "essential_02", "essential").await
        }
        "Character.buyGanMaterial" => {
            crate::economy::buy_essential(pool, args, "material_1001", "gan").await
        }
        "Character.sellItem" => {
            crate::economy::sell_item(
                pool,
                if args.len() == 1 {
                    args[0].as_array().map(Vec::as_slice).unwrap_or(args)
                } else {
                    args
                },
            )
            .await
        }
        "Character.buySkill" => crate::economy::buy_skill(pool, args).await,
        "AdvanceAcademy.upgradeSkill" => crate::academy::upgrade(pool, args).await,
        "Character.changeSpecialClass" => crate::academy::change_class(pool, args).await,
        "Character.useItem" => crate::consumables::use_item(pool, args, false).await,
        "Character.useBattleItem" => crate::consumables::use_item(pool, args, true).await,
        "Blacksmith.forgeItem" => crate::crafting::forge(pool, args).await,
        "Character.resetTalents" => crate::talents::dispatch(pool, "resetTalents", args).await,
        "Character.resetElements" | "Character.resetSenjutsu" => {
            crate::resets::dispatch(pool, target.split_once('.').unwrap().1, args).await
        }
        "Character.buySenjutsuEssential" => {
            crate::economy::buy_essential(pool, args, "essential_11", "essential").await
        }
        target if target.starts_with("Senjutsu.") => {
            crate::senjutsu::dispatch(pool, target.strip_prefix("Senjutsu.").unwrap(), args).await
        }
        target if target.starts_with("Talent.") => {
            crate::talents::dispatch(pool, target.strip_prefix("Talent.").unwrap(), args).await
        }
        "Character.getSkillSets" => crate::skillsets::list(pool, args).await,
        "Character.getGearPresets"
        | "Character.unlockGearPresetSlot"
        | "Character.updateGearPreset"
        | "Character.renameGearPreset" => {
            crate::gear_presets::dispatch(pool, target.split_once('.').unwrap().1, args).await
        }
        "Character.createSkillSet" => crate::skillsets::create(pool, args).await,
        "Character.saveSkillSet" => crate::skillsets::save(pool, args).await,
        "BattleSystem.startMission" => crate::missions::start(state, pool, args).await,
        "BattleSystem.getMissionSData" => crate::missions::s_data(pool, args).await,
        "BattleSystem.finishMission" => crate::missions::finish(state, pool, args).await,
        "Character.getMissionRoomData" => crate::recruits::room(pool, args).await,
        "Character.recruitTeammate" => crate::recruits::recruit(pool, args).await,
        "Character.removeRecruitments" => crate::recruits::remove(pool, args).await,
        "Events.get" => crate::events::get(pool, args).await,
        "Events.executeService" => crate::events::execute_service(pool, args).await,
        target if target.starts_with("PackageEvent.") => {
            crate::events::dispatch_package_event(
                pool,
                target.strip_prefix("PackageEvent.").unwrap(),
                args,
            )
            .await
        }
        target if target.starts_with("Giveaway.") => {
            crate::events::dispatch_giveaway(pool, target.strip_prefix("Giveaway.").unwrap(), args)
                .await
        }
        "Leaderboard.getData" => crate::events::leaderboard_get_data(pool, args).await,
        target if target.starts_with("PvP.") => {
            crate::events::dispatch_pvp(pool, target.strip_prefix("PvP.").unwrap(), args).await
        }
        target if target.starts_with("MonsterHunterEvent2023.") => {
            crate::seasonal::dispatch_monster_hunter(
                state,
                pool,
                target.strip_prefix("MonsterHunterEvent2023.").unwrap(),
                args,
            )
            .await
        }
        target if target.starts_with("DragonHuntEvent.") => {
            crate::seasonal::dispatch_dragon_hunt(
                state,
                pool,
                target.strip_prefix("DragonHuntEvent.").unwrap(),
                args,
            )
            .await
        }
        target if target.starts_with("JusticeBadgeEvent2024.") => {
            crate::seasonal::dispatch_justice_badge(
                pool,
                target.strip_prefix("JusticeBadgeEvent2024.").unwrap(),
                args,
            )
            .await
        }
        target if target.starts_with("HalloweenEvent2025.") => {
            crate::seasonal::dispatch_halloween_2025(
                state,
                pool,
                target.strip_prefix("HalloweenEvent2025.").unwrap(),
                args,
            )
            .await
        }
        target if target.starts_with("ConfrontingDeathEvent2025.") => {
            crate::seasonal::dispatch_confronting_death_2025(
                state,
                pool,
                target.strip_prefix("ConfrontingDeathEvent2025.").unwrap(),
                args,
            )
            .await
        }
        target if target.starts_with("ThanksGivingEvent2025.") => {
            crate::seasonal::dispatch_thanksgiving_2025(
                state,
                pool,
                target.strip_prefix("ThanksGivingEvent2025.").unwrap(),
                args,
            )
            .await
        }
        target if target.starts_with("ChristmasEvent2021.") => {
            crate::seasonal::dispatch_christmas_2021(
                state,
                pool,
                target.strip_prefix("ChristmasEvent2021.").unwrap(),
                args,
            )
            .await
        }
        target if target.starts_with("ChristmasEvent2025.") => {
            crate::seasonal::dispatch_christmas_2025(
                state,
                pool,
                target.strip_prefix("ChristmasEvent2025.").unwrap(),
                args,
            )
            .await
        }
        target if target.starts_with("NewYear2026.") => {
            crate::seasonal::dispatch_new_year_2026(
                pool,
                target.strip_prefix("NewYear2026.").unwrap(),
                args,
            )
            .await
        }
        "Character.getInfo" => characters::info(pool, args).await,
        "Character.deleteCharacter" => characters::delete_character(pool, args).await,
        "Character.convertTokensToGold" => characters::convert_tokens_to_gold(pool, args).await,
        "Character.renameCharacter" => characters::rename_character(pool, args).await,
        "Character.addItems" => characters::dev_add_items(pool, args).await,
        "Character.setCharInfo" => characters::dev_set_char_info(pool, args).await,
        "Character.toggleEmblem" => characters::dev_toggle_emblem(pool, args).await,
        "Character.learnSkill" => characters::dev_learn_skill(pool, args).await,
        "Pet.executeService" => {
            let method = string(args, 0);
            if let Some(params) = args.get(1).and_then(Json::as_array) {
                crate::pets::dispatch(pool, &method, params).await
            } else {
                Ok(session_error("Invalid pet parameters"))
            }
        }
        target if target.starts_with("Pet.") => {
            crate::pets::dispatch(pool, target.strip_prefix("Pet.").unwrap(), args).await
        }
        target if target.starts_with("PetCombination.") => {
            crate::pets::dispatch_combination(
                state,
                pool,
                target.strip_prefix("PetCombination.").unwrap(),
                args,
            )
            .await
        }
        target if target.starts_with("DailyReward.") => {
            crate::daily::dispatch_daily_reward(
                pool,
                target.strip_prefix("DailyReward.").unwrap(),
                args,
            )
            .await
        }
        target if target.starts_with("DailyRoulette.") => {
            crate::daily::dispatch_daily_roulette(
                pool,
                target.strip_prefix("DailyRoulette.").unwrap(),
                args,
            )
            .await
        }
        target if target.starts_with("DailyScratch.") => {
            crate::daily::dispatch_daily_scratch(
                pool,
                target.strip_prefix("DailyScratch.").unwrap(),
                args,
            )
            .await
        }
        target if target.starts_with("WelcomeLogin.") => {
            crate::daily::dispatch_welcome_login(
                pool,
                target.strip_prefix("WelcomeLogin.").unwrap(),
                args,
            )
            .await
        }
        target
            if target.starts_with("ChuninExam.")
                || target.starts_with("JouninExam.")
                || target.starts_with("NinjaTutorExam.")
                || target.starts_with("SpecialJouninExam.") =>
        {
            let (service, method) = target.split_once('.').unwrap();
            crate::exams::dispatch(pool, service, method, args).await
        }
        target if target.starts_with("HuntingHouse.") => {
            crate::hunting::dispatch_hunting_house(
                state,
                pool,
                target.strip_prefix("HuntingHouse.").unwrap(),
                args,
            )
            .await
        }
        target if target.starts_with("EudemonGarden.") => {
            crate::hunting::dispatch_eudemon_garden(
                state,
                pool,
                target.strip_prefix("EudemonGarden.").unwrap(),
                args,
            )
            .await
        }
        target if target.starts_with("MaterialMarket.") => {
            crate::markets::dispatch_material_market(
                pool,
                target.strip_prefix("MaterialMarket.").unwrap(),
                args,
            )
            .await
        }
        target if target.starts_with("MysteriousMarket.") => {
            crate::markets::dispatch_mysterious_market(
                pool,
                target.strip_prefix("MysteriousMarket.").unwrap(),
                args,
            )
            .await
        }
        target if target.starts_with("SpecialDeals.") => {
            crate::markets::dispatch_special_deals(
                pool,
                target.strip_prefix("SpecialDeals.").unwrap(),
                args,
            )
            .await
        }
        target if target.starts_with("ExoticPackage.") => {
            crate::markets::dispatch_exotic_package(
                pool,
                target.strip_prefix("ExoticPackage.").unwrap(),
                args,
            )
            .await
        }
        target if target.starts_with("Friend.") => {
            crate::friends::dispatch(pool, target.strip_prefix("Friend.").unwrap(), args).await
        }
        "Mail.executeService" => {
            let method = string(args, 0);
            if let Some(params) = args.get(1).and_then(Json::as_array) {
                crate::mail::dispatch(pool, &method, params).await
            } else {
                Ok(session_error("Invalid mail parameters"))
            }
        }
        target if target.starts_with("Mail.") => {
            crate::mail::dispatch(pool, target.strip_prefix("Mail.").unwrap(), args).await
        }
        "ShadowWar.executeService" => {
            let method = string(args, 0);
            if let Some(params) = args.get(1).and_then(Json::as_array) {
                crate::shadow_war::dispatch(state, pool, &method, params).await
            } else {
                Ok(session_error("Invalid shadow war parameters"))
            }
        }
        target if target.starts_with("ShadowWar.") => {
            crate::shadow_war::dispatch(
                state,
                pool,
                target.strip_prefix("ShadowWar.").unwrap(),
                args,
            )
            .await
        }
        target if target.starts_with("Clan.") => {
            crate::shadow_war::dispatch_clan_or_crew(
                pool,
                target.strip_prefix("Clan.").unwrap(),
                args,
            )
            .await
        }
        target if target.starts_with("Crew.") => {
            crate::shadow_war::dispatch_clan_or_crew(
                pool,
                target.strip_prefix("Crew.").unwrap(),
                args,
            )
            .await
        }
        _ => {
            tracing::warn!(method = target, "AMF method not implemented");
            Ok(json!({"status":500,"error":format!("{target} failed: method is not implemented")}))
        }
    };
    result.unwrap_or_else(|error| {
        tracing::error!(method=target,error=%error,"AMF action failed");
        #[cfg(test)]
        eprintln!("{target}: {error}");
        #[cfg(not(test))]
        let _ = error;
        json!({"status":500,"error":format!("{target} failed: database operation failed")})
    })
}

async fn register(pool: &MySqlPool, args: &[Json]) -> Result<Json, sqlx::Error> {
    let username = string(args, 0);
    let email = string(args, 1);
    let password = string(args, 2);
    if username.is_empty()
        || username.len() > 255
        || email.is_empty()
        || email.len() > 255
        || password.is_empty()
        || password.len() > 72
    {
        return Ok(json!({"status":2,"result":"Invalid registration details"}));
    }
    for (field, value, message) in [
        ("username", &username, "Username already exists!"),
        ("email", &email, "Email already exists!"),
    ] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM users WHERE {field}=?"))
            .bind(value)
            .fetch_one(pool)
            .await?;
        if count > 0 {
            return Ok(json!({"status":2,"result":message}));
        }
    }
    let hash = tokio::task::spawn_blocking(move || auth::hash_password(&password))
        .await
        .ok()
        .and_then(Result::ok);
    let Some(hash) = hash else {
        return Ok(json!({"status":0,"error":"Internal Server Error"}));
    };
    match sqlx::query("INSERT INTO users (name,username,email,password,created_at,updated_at) VALUES (?,?,?,?,NOW(),NOW())").bind(&username).bind(&username).bind(&email).bind(hash).execute(pool).await {
        Ok(_) => Ok(json!({"status":1,"result":"Registered Successfully!"})),
        Err(error) if error.as_database_error().is_some_and(|e|e.is_unique_violation()) => Ok(json!({"status":2,"result":"Username or email already exists!"})),
        Err(error) => Err(error),
    }
}
async fn login(pool: &MySqlPool, args: &[Json]) -> Result<Json, sqlx::Error> {
    let Some(user) = sqlx::query("SELECT id,password FROM users WHERE username=?")
        .bind(string(args, 0))
        .fetch_optional(pool)
        .await?
    else {
        return Ok(json!({"status":2}));
    };
    let Some(password) =
        auth::decrypt_password(&string(args, 1), &string(args, 5), &string(args, 2))
    else {
        return Ok(json!({"status":2}));
    };
    let hash: String = user.try_get("password")?;
    let valid = tokio::task::spawn_blocking(move || auth::verify_password(&password, &hash))
        .await
        .unwrap_or(false);
    if !valid {
        return Ok(json!({"status":2}));
    }
    let uid: u64 = user.try_get("id")?;
    let Ok(session) = auth::random_hex() else {
        return Ok(json!({"status":500,"error":"Entropy unavailable"}));
    };
    sqlx::query("UPDATE users SET session_key=?,updated_at=NOW() WHERE id=?")
        .bind(&session)
        .bind(uid)
        .execute(pool)
        .await?;
    let (sha, md5) = auth::hashes(uid, &session);
    let mut seasons = Vec::new();
    for (table, column) in [
        ("clan_seasons", "number"),
        ("crew_seasons", "number"),
        ("shadow_war_seasons", "num"),
    ] {
        let query = format!(
            "SELECT CAST({column} AS CHAR) FROM {table} WHERE active=1 ORDER BY {column} DESC LIMIT 1"
        );
        seasons.push(
            sqlx::query_scalar::<_, String>(&query)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
                .unwrap_or_else(|| "1".into()),
        );
    }
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string();
    Ok(
        json!({"status":1,"error":0,"uid":uid,"sessionkey":session,"hash":sha,"hash_md5":md5,"sessionkey_b64":auth::base64(&session),"__":string(args,5),"events":["welcome_bonus","mysterious-market","chunin_package","special-deals","monster_hunter_2023","dragon_hunt_2024","justice-badge2024","giveaway-center","leaderboard","tailedbeast","dailygacha","dragongacha","exoticpackage","thanksgiving2025","elementalars","xmass2025","valentine2026","phantom_kyunoki_2026"],"clan_season":seasons[0],"crew_season":seasons[1],"sw_season":seasons[2],"banners":[[]],"system_time":auth::base64(time)}),
    )
}

pub async fn guard(
    pool: &MySqlPool,
    id: i64,
    key: &str,
    character: bool,
) -> Result<Option<Json>, sqlx::Error> {
    guard_kind(pool, id, key, character, false).await
}

pub async fn registration_guard(
    pool: &MySqlPool,
    id: i64,
    key: &str,
) -> Result<Option<Json>, sqlx::Error> {
    guard_kind(pool, id, key, false, true).await
}

async fn guard_kind(
    pool: &MySqlPool,
    id: i64,
    key: &str,
    character: bool,
    registration: bool,
) -> Result<Option<Json>, sqlx::Error> {
    if id <= 0 || key.trim().is_empty() {
        return Ok(Some(session_error("Character id or session key is empty")));
    }
    let query = if character {
        "SELECT u.id,u.session_key FROM users u JOIN characters c ON c.user_id=u.id WHERE c.id=?"
    } else {
        "SELECT id,session_key FROM users WHERE id=?"
    };
    let Some(row) = sqlx::query(query).bind(id).fetch_optional(pool).await? else {
        return Ok(Some(session_error(if character {
            "Character not found"
        } else {
            "User not found"
        })));
    };
    let uid: u64 = row.try_get("id")?;
    let stored: Option<String> = row.try_get("session_key")?;
    Ok(
        if if registration {
            auth::matches_registration_session(uid, stored.as_deref().unwrap_or(""), key)
        } else {
            auth::matches_session(uid, stored.as_deref().unwrap_or(""), key)
        } {
            None
        } else {
            Some(session_error("Invalid session key"))
        },
    )
}
