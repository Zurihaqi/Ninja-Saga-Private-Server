//! Headless, client-shaped AMF calls against the synthetic account owned by the parent test.
use super::amf_call;
use axum::Router;
use persistence::{MySqlPool, Row, sqlx};
use serde_json::json;

pub async fn progression_and_companions(app: &Router, pool: &MySqlPool, id: u64, key: &str) {
    assert_eq!(
        amf_call(app, "ChuninExam.startStage", json!([key, id, 1])).await["status"],
        2
    );
    for (service, level, first, count, rank, promotion) in [
        ("ChuninExam", 20, 1, 5, 3, "promoteToChunin"),
        ("JouninExam", 40, 6, 5, 5, "promoteToJounin"),
        ("SpecialJouninExam", 60, 11, 13, 7, "promoteToSpecialJounin"),
        ("NinjaTutorExam", 80, 24, 12, 9, "promoteToNinjaTutor"),
    ] {
        sqlx::query("UPDATE characters SET level=? WHERE id=?")
            .bind(level)
            .bind(id)
            .execute(pool)
            .await
            .unwrap();
        let data = amf_call(app, &format!("{service}.getData"), json!([key, id])).await;
        assert_eq!(
            data["data"].as_array().unwrap().len(),
            count as usize,
            "{data}"
        );
        assert_eq!(
            amf_call(
                app,
                &format!("{service}.startStage"),
                json!([key, id, first + 1])
            )
            .await["status"],
            2
        );
        assert_eq!(
            amf_call(app, &format!("{service}.{promotion}"), json!([key, id])).await["status"],
            2
        );
        for stage in first..first + count {
            let start = amf_call(
                app,
                &format!("{service}.startStage"),
                json!([key, id, stage]),
            )
            .await;
            assert_eq!(start["status"], 1, "{service} stage {stage}: {start}");
            let expected = if rank == 9 {
                stage
            } else if rank == 7 {
                stage - 5
            } else {
                stage - first + 1
            };
            assert_eq!(start["result"], format!("start_stage_{expected}"));
            let finish = amf_call(
                app,
                &format!("{service}.finishStage"),
                json!([key, id, stage]),
            )
            .await;
            assert_eq!(finish["status"], 1, "{finish}");
        }
        let claim_args = if rank == 7 {
            json!([key, id, "skill_4000"])
        } else {
            json!([key, id])
        };
        let target = format!("{service}.{promotion}");
        let (a, b) = tokio::join!(
            amf_call(app, &target, claim_args.clone()),
            amf_call(app, &target, claim_args)
        );
        assert_eq!(a["status"], 1, "{a}");
        assert_eq!(b["status"], 1, "{b}");
        assert_eq!(
            [&a, &b]
                .iter()
                .filter(|v| !v["rewards"].as_array().unwrap().is_empty())
                .count(),
            1,
            "{a} {b}"
        );
        let loaded = amf_call(app, "SystemLogin.getCharacterData", json!([id, key])).await;
        assert_eq!(loaded["character_data"]["character_rank"], rank);
        if rank == 7 {
            assert_eq!(loaded["character_data"]["character_class"], "skill_4000");
        }
    }
    // Give only this synthetic account the budget required to exercise catalog purchases.
    sqlx::query("UPDATE characters SET gold=10000000 WHERE id=?")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE users u JOIN characters c ON c.user_id=u.id SET u.tokens=10000000 WHERE c.id=?",
    )
    .bind(id)
    .execute(pool)
    .await
    .unwrap();
    let pet_type: String =
        sqlx::query_scalar("SELECT pet_id FROM pets ORDER BY price_gold,price_tokens LIMIT 1")
            .fetch_one(pool)
            .await
            .unwrap();
    let buy = amf_call(app, "PetService.buyPet", json!([id, key, pet_type])).await;
    assert_eq!(buy["status"], 1, "{buy}");
    let pets = amf_call(
        app,
        "PetService.executeService",
        json!(["getPets", [id, key]]),
    )
    .await;
    let pet = pets["pets"][0]["pet_id"].as_u64().unwrap();
    assert_eq!(
        amf_call(app, "PetService.equipPet", json!([id, key, pet])).await["status"],
        1
    );
    let loaded = amf_call(app, "SystemLogin.getCharacterData", json!([id, key])).await;
    assert_eq!(loaded["pet_data"]["pet_id"], pet, "{loaded}");
    assert_eq!(
        amf_call(app, "PetService.releasePet", json!([id, key, pet])).await["status"],
        0
    );
    assert_eq!(
        amf_call(
            app,
            "PetService.renamePet",
            json!([id, key, pet, "Renamed"])
        )
        .await["status"],
        2
    );
    sqlx::query("INSERT INTO character_items (character_id,item_id,category,quantity) VALUES (?,'essential_01','essential',3)").bind(id).execute(pool).await.unwrap();
    assert_eq!(
        amf_call(
            app,
            "PetService.renamePet",
            json!([id, key, pet, "Renamed"])
        )
        .await["status"],
        1
    );
    assert_eq!(
        amf_call(app, "PetService.equipPet", json!([id, key, pet + 999999])).await["status"],
        0
    );
    let npc: String =
        sqlx::query_scalar("SELECT npc_id FROM npcs ORDER BY price_gold,price_tokens LIMIT 1")
            .fetch_one(pool)
            .await
            .unwrap();
    let recruit = amf_call(
        app,
        "CharacterService.recruitTeammate",
        json!([id, key, npc]),
    )
    .await;
    assert_eq!(recruit["status"], 1, "{recruit}");
    let room = amf_call(app, "CharacterService.getMissionRoomData", json!([id, key])).await;
    assert_eq!(room["recruit"][0]["id"]["recruiter_id"], npc);
    let before: i32 = sqlx::query_scalar("SELECT gold FROM characters WHERE id=?")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(
        amf_call(
            app,
            "CharacterService.recruitTeammate",
            json!([id, key, npc])
        )
        .await["status"],
        2
    );
    assert_eq!(
        sqlx::query_scalar::<_, i32>("SELECT gold FROM characters WHERE id=?")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap(),
        before
    );
    let enemy = sqlx::query("SELECT hp,agility FROM enemies WHERE enemy_id='ene_01'")
        .fetch_one(pool)
        .await
        .unwrap();
    let stats = format!(
        "id:ene_01|hp:{}|agility:{}",
        enemy.try_get::<i32, _>("hp").unwrap(),
        enemy.try_get::<i32, _>("agility").unwrap()
    );
    let token = amf_call(
        app,
        "BattleSystem.startMission",
        json!([id, "msn_1", "ene_01", stats, "", "", key]),
    )
    .await;
    let result = amf_call(
        app,
        "BattleSystem.finishMission",
        json!([id, "msn_1", token, "", "", key]),
    )
    .await;
    assert_eq!(result["status"], 1, "{result}");
    let fresh = crate::configured_router(crate::GameState::with_database(Some(pool.clone())));
    crafting_and_consumables(&fresh, pool, id, key).await;
    advanced_progression(&fresh, pool, id, key).await;
    let pets = amf_call(&fresh, "PetService.getPets", json!([id, key])).await;
    assert_eq!(pets["pets"][0]["pet_name"], "Renamed");
    assert_eq!(pets["pets"][0]["pet_xp"], 20);
    super::player_actions::run(&fresh, pool, id, key, pet).await;
    assert_eq!(
        amf_call(
            &fresh,
            "CharacterService.getMissionRoomData",
            json!([id, key])
        )
        .await["recruit"],
        json!([])
    );
    assert_eq!(
        amf_call(&fresh, "PetService.unequipPet", json!([id, key])).await["status"],
        1
    );
    assert_eq!(
        amf_call(&fresh, "PetService.releasePet", json!([id, key, pet])).await["status"],
        1
    );
    assert_eq!(
        amf_call(&fresh, "PetService.getPets", json!([id, key])).await["pets"],
        json!([])
    );
}

async fn advanced_progression(app: &Router, pool: &MySqlPool, id: u64, key: &str) {
    assert_eq!(
        amf_call(
            app,
            "TalentService.upgradeSkill",
            json!([id, key, "skill_1006", false])
        )
        .await["status"],
        2
    );
    let discovered = amf_call(
        app,
        "TalentService.discoverTalent",
        json!([id, key, "Extreme", "de"]),
    )
    .await;
    assert_eq!(discovered["status"], 1, "{discovered}");
    assert_eq!(discovered["newt"], 1);
    assert_eq!(
        amf_call(
            app,
            "TalentService.discoverTalent",
            json!([id, key, "Extreme", "de"])
        )
        .await["status"],
        2
    );
    assert_eq!(
        amf_call(app, "TalentService.buyPackageTP", json!([id, key, 0])).await["status"],
        1
    );
    assert_eq!(
        amf_call(
            app,
            "TalentService.upgradeSkill",
            json!([id, key, "skill_1006", false])
        )
        .await["current_tp"],
        20
    );
    assert_eq!(
        amf_call(
            app,
            "TalentService.upgradeSkill",
            json!([id, key, "skill_1006", true])
        )
        .await["current_tp"],
        10
    );
    let talents = amf_call(app, "TalentService.getTalentSkills", json!([id, key])).await;
    assert_eq!(
        talents["data"][0],
        json!({"item_id":"skill_1006","item_level":2,"talent_type":"de"})
    );
    let mut reset_tx = pool.begin().await.unwrap();
    crate::inventory::give(&mut reset_tx, id as i64, "essential_02", 5)
        .await
        .unwrap();
    reset_tx.commit().await.unwrap();
    assert_eq!(
        amf_call(
            app,
            "CharacterService.resetTalents",
            json!([id, key, ["de"]])
        )
        .await["status"],
        1
    );
    assert_eq!(
        amf_call(app, "TalentService.getTalentSkills", json!([id, key])).await["data"],
        json!([])
    );
    assert_eq!(
        amf_call(
            app,
            "SenjutsuService.upgradeSkill",
            json!([id, key, "skill_3001", false])
        )
        .await["status"],
        2
    );
    let discovered = amf_call(
        app,
        "SenjutsuService.discoverSenjutsu",
        json!([id, key, "toad"]),
    )
    .await;
    assert_eq!(discovered["status"], 1, "{discovered}");
    assert_eq!(
        amf_call(
            app,
            "SenjutsuService.discoverSenjutsu",
            json!([id, key, "snake"])
        )
        .await["status"],
        2
    );
    sqlx::query("UPDATE characters SET ss=50 WHERE id=?")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        amf_call(
            app,
            "SenjutsuService.upgradeSkill",
            json!([id, key, "skill_3001", true])
        )
        .await["spent_ss"],
        40
    );
    assert_eq!(
        amf_call(
            app,
            "SenjutsuService.equipSkill",
            json!([id, key, "skill_3001,skill_unknown"])
        )
        .await["skills"],
        "skill_3001"
    );
    let data = amf_call(app, "SenjutsuService.getSenjutsuSkills", json!([id, key])).await;
    assert_eq!(
        data["data"][0],
        json!({"id":"skill_3001","level":3,"type":"toad"})
    );
    assert_eq!(data["ss"], 10);
    assert_eq!(
        amf_call(
            app,
            "CharacterService.changeSpecialClass",
            json!([id, key, "skill_4001"])
        )
        .await["status"],
        1
    );
    assert_eq!(
        amf_call(
            app,
            "CharacterService.changeSpecialClass",
            json!([id, key, "skill_4001"])
        )
        .await["status"],
        2
    );
    sqlx::query("INSERT INTO character_skills (character_id,skill_id) VALUES (?,'skill_152')")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE characters SET equipment_skills='skill_152' WHERE id=?")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    let upgrade = amf_call(
        app,
        "AdvanceAcademyService.upgradeSkill",
        json!([id, key, "skill_232"]),
    )
    .await;
    assert_eq!(upgrade["status"], 1, "{upgrade}");
    assert_eq!(upgrade["character_set_skills"], "skill_232");
    assert_eq!(
        amf_call(
            app,
            "AdvanceAcademyService.upgradeSkill",
            json!([id, key, "skill_232"])
        )
        .await["status"],
        2
    );
    let fresh = crate::configured_router(crate::GameState::with_database(Some(pool.clone())));
    let loaded = amf_call(&fresh, "SystemLogin.getCharacterData", json!([id, key])).await;
    assert_eq!(loaded["character_sets"]["skills"], "skill_232");
    assert_eq!(loaded["character_sets"]["senjutsu_skills"], "skill_3001");
    assert_eq!(loaded["character_data"]["character_class"], "skill_4001");
    assert_eq!(
        loaded["character_data"]["character_talent_1"],
        serde_json::Value::Null
    );
}

async fn crafting_and_consumables(app: &Router, pool: &MySqlPool, id: u64, key: &str) {
    let before: i32 = sqlx::query_scalar("SELECT gold FROM characters WHERE id=?")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap();
    let missing = amf_call(
        app,
        "BlacksmithService.forgeItem",
        json!([id, key, "wpn_609", "gold"]),
    )
    .await;
    assert_eq!(missing["status"], 2, "{missing}");
    assert_eq!(
        sqlx::query_scalar::<_, i32>("SELECT gold FROM characters WHERE id=?")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap(),
        before
    );
    sqlx::query("INSERT INTO character_items (character_id,item_id,category,quantity) VALUES (?,'wpn_03','weapon',1),(?,'item_01','item',1),(?,'essential_03','essential',1),(?,'essential_12','essential',1)").bind(id).bind(id).bind(id).bind(id).execute(pool).await.unwrap();
    sqlx::query("UPDATE characters SET equipment_weapon='wpn_03' WHERE id=?")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    let forged = amf_call(
        app,
        "BlacksmithService.forgeItem",
        json!([id, key, "wpn_609", "gold"]),
    )
    .await;
    assert_eq!(forged["status"], 1, "{forged}");
    assert_eq!(
        forged["requirements"],
        json!([["material_01", "wpn_03"], [1, 1]])
    );
    let loaded = amf_call(app, "SystemLogin.getCharacterData", json!([id, key])).await;
    assert_eq!(loaded["character_data"]["character_gold"], before - 1000);
    assert_eq!(loaded["character_sets"]["weapon"], "wpn_01");
    assert!(
        !loaded["character_inventory"]["char_weapons"]
            .as_str()
            .unwrap()
            .contains("wpn_03:")
    );
    assert!(
        loaded["character_inventory"]["char_weapons"]
            .as_str()
            .unwrap()
            .contains("wpn_609:1")
    );
    assert_eq!(
        amf_call(
            app,
            "BlacksmithService.forgeItem",
            json!([id, key, "wpn_609", "gold"])
        )
        .await["status"],
        2
    );
    assert_eq!(
        sqlx::query_scalar::<_, i32>(
            "SELECT quantity FROM character_items WHERE character_id=? AND item_id='material_01'"
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap(),
        4
    );
    assert_eq!(
        amf_call(
            app,
            "CharacterService.useBattleItem",
            json!([key, id, "item_01"])
        )
        .await["status"],
        1
    );
    assert_eq!(
        amf_call(
            app,
            "CharacterService.useBattleItem",
            json!([key, id, "item_01"])
        )
        .await["status"],
        2
    );
    assert_eq!(
        amf_call(
            app,
            "CharacterService.useItem",
            json!([id, key, "essential_03"])
        )
        .await["rewards"],
        json!(["tp_5"])
    );
    assert_eq!(
        amf_call(
            app,
            "CharacterService.useItem",
            json!([id, key, "essential_03"])
        )
        .await["status"],
        2
    );
    assert_eq!(
        sqlx::query_scalar::<_, i32>("SELECT tp FROM characters WHERE id=?")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap(),
        5
    );
    let pack = amf_call(
        app,
        "CharacterService.useItem",
        json!([id, key, "essential_12"]),
    )
    .await;
    assert_eq!(pack["status"], 1, "{pack}");
    assert_eq!(
        sqlx::query_scalar::<_, i32>(
            "SELECT quantity FROM character_items WHERE character_id=? AND item_id='set_831_0'"
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap(),
        1
    );
    let events_boot = amf_call(app, "EventsService.get", json!(null)).await;
    assert_eq!(events_boot["status"], 1, "{events_boot}");
    assert!(events_boot["events"]["seasonal"].is_array());
    let events_auth = amf_call(app, "EventsService.get", json!([id, key])).await;
    assert_eq!(events_auth["status"], 1, "{events_auth}");
    assert_eq!(
        amf_call(app, "EventsService.get", json!([id, "wrong"])).await["status"],
        0
    );
    hunting_and_markets(app, pool, id, key).await;
}

async fn hunting_and_markets(app: &Router, pool: &MySqlPool, id: u64, key: &str) {
    // HuntingHouse: getData, dailyClaim, buyMaterial, startHunting, finishHunting, getItems, forgeItem
    let hh_data = amf_call(app, "HuntingHouse.getData", json!([id, key])).await;
    assert_eq!(hh_data["status"], 1, "{hh_data}");
    assert_eq!(hh_data["daily_claim"], false);

    let hh_claim = amf_call(app, "HuntingHouse.dailyClaim", json!([id, key])).await;
    assert_eq!(hh_claim["status"], 1, "{hh_claim}");
    assert_eq!(
        amf_call(app, "HuntingHouse.dailyClaim", json!([id, key])).await["status"],
        2
    );
    let hh_buy = amf_call(app, "HuntingHouse.buyMaterial", json!([id, key, 5])).await;
    assert_eq!(hh_buy["status"], 1, "{hh_buy}");

    let hh_start = amf_call(app, "HuntingHouse.startHunting", json!([id, 2, key])).await;
    assert_eq!(hh_start["status"], 1, "{hh_start}");
    let hh_code = hh_start["code"].as_str().unwrap();
    let hh_finish = amf_call(
        app,
        "HuntingHouse.finishHunting",
        json!([id, 2, hh_code, hh_start["hash"], key, null]),
    )
    .await;
    assert_eq!(hh_finish["status"], 1, "{hh_finish}");
    assert_eq!(
        hh_finish["result"][2],
        json!(["material_509", "material_509"])
    );
    assert_eq!(
        amf_call(
            app,
            "HuntingHouse.finishHunting",
            json!([id, 2, hh_code, hh_start["hash"], key, null])
        )
        .await["status"],
        0
    );

    let hh_items = amf_call(app, "HuntingHouse.getItems", json!([id, key])).await;
    assert_eq!(hh_items["status"], 1, "{hh_items}");
    assert!(!hh_items["items"].as_array().unwrap().is_empty());

    let hh_forge = amf_call(app, "HuntingHouse.forgeItem", json!([id, key, "wpn_81"])).await;
    assert_eq!(hh_forge["status"], 1, "{hh_forge}");
    assert_eq!(hh_forge["item"], "wpn_81");

    // EudemonGarden: getData, startHunting, finishHunting, buyTries (note: getData/buyTries take sessionKey first)
    let eg_data = amf_call(app, "EudemonGarden.getData", json!([key, id])).await;
    assert_eq!(eg_data["status"], 1, "{eg_data}");
    assert!(!eg_data["data"].as_str().unwrap().is_empty());

    let eg_start = amf_call(app, "EudemonGarden.startHunting", json!([id, 0, key])).await;
    assert_eq!(eg_start["status"], 1, "{eg_start}");
    let eg_code = eg_start["code"].as_str().unwrap();
    let eg_finish = amf_call(
        app,
        "EudemonGarden.finishHunting",
        json!([id, 0, eg_code, eg_start["hash"], key, null]),
    )
    .await;
    assert_eq!(eg_finish["status"], 1, "{eg_finish}");
    assert_eq!(
        amf_call(
            app,
            "EudemonGarden.finishHunting",
            json!([id, 0, eg_code, eg_start["hash"], key, null])
        )
        .await["status"],
        0
    );

    let eg_buy = amf_call(app, "EudemonGarden.buyTries", json!([key, id])).await;
    assert_eq!(eg_buy["status"], 1, "{eg_buy}");

    // MaterialMarket: getItems, forgeItem
    let mm_items = amf_call(app, "MaterialMarket.getItems", json!([id, key])).await;
    assert_eq!(mm_items["status"], 1, "{mm_items}");
    assert!(!mm_items["items"].as_array().unwrap().is_empty());

    sqlx::query("INSERT INTO character_items (character_id,item_id,category,quantity) VALUES (?,'material_01','material',10)")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    let mm_forge = amf_call(app, "MaterialMarket.forgeItem", json!([id, key, "wpn_609"])).await;
    assert_eq!(mm_forge["status"], 1, "{mm_forge}");
    assert_eq!(mm_forge["item"], "wpn_609");

    // SpecialDeals: getDeals, buy
    let deals = amf_call(app, "SpecialDeals.getDeals", json!([id, key])).await;
    assert_eq!(deals["status"], 1, "{deals}");
    if let Some(first_deal) = deals["deals"].as_array().and_then(|d| d.first()) {
        let deal_id = first_deal["id"].as_i64().unwrap();
        let deal_buy = amf_call(app, "SpecialDeals.buy", json!([id, key, deal_id])).await;
        assert_eq!(deal_buy["status"], 1, "{deal_buy}");
    }

    // ExoticPackage: get, buy
    let exotic = amf_call(app, "ExoticPackage.get", json!([id, key])).await;
    assert_eq!(exotic["status"], 1, "{exotic}");
    if let Some((pkg_key, _)) = exotic["packages"].as_object().and_then(|o| o.iter().next()) {
        let pkg_buy = amf_call(app, "ExoticPackage.buy", json!([id, key, pkg_key])).await;
        assert_eq!(pkg_buy["status"], 1, "{pkg_buy}");
    }

    // MysteriousMarket: closed state and active state
    sqlx::query("DELETE FROM mysterious_markets WHERE discount='99'")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO mysterious_markets (active, ends_at, discount, refresh_cost, refresh_max, items, all_packages, created_at, updated_at) VALUES (1, DATE_ADD(NOW(), INTERVAL 1 HOUR), '99', 50, 2, '[{\"code\":\"skill_9001\",\"prices\":[100,80]},{\"code\":\"skill_9002\",\"prices\":[200,150]}]', '[{\"code\":\"skill_9001\",\"prices\":[100,80]},{\"code\":\"skill_9002\",\"prices\":[200,150]},{\"code\":\"skill_9003\",\"prices\":[300,250]}]', NOW(), NOW())",
    )
    .execute(pool)
    .await
    .unwrap();

    let mys_data = amf_call(app, "MysteriousMarket.getPackageData", json!([id, key])).await;
    assert_eq!(mys_data["status"], 1, "{mys_data}");
    assert!(mys_data["end_time"].as_i64().unwrap_or(0) > 0);

    // Sequential check: buying index 1 before index 0 fails with status 2
    assert_eq!(
        amf_call(
            app,
            "MysteriousMarket.buyPackage",
            json!([id, key, "skill_9002"])
        )
        .await["status"],
        2
    );
    assert_eq!(
        amf_call(
            app,
            "MysteriousMarket.buyPackage",
            json!([id, key, "skill_9001"])
        )
        .await["status"],
        1
    );
    assert_eq!(
        amf_call(
            app,
            "MysteriousMarket.buyPackage",
            json!([id, key, "skill_9002"])
        )
        .await["status"],
        1
    );
    let mys_all = amf_call(app, "MysteriousMarket.getAllPackagesList", json!([id, key])).await;
    assert_eq!(mys_all["status"], 1, "{mys_all}");
    let mys_refresh = amf_call(app, "MysteriousMarket.refreshPackage", json!([id, key])).await;
    assert_eq!(mys_refresh["status"], 1, "{mys_refresh}");
    sqlx::query("DELETE FROM mysterious_markets WHERE discount='99'")
        .execute(pool)
        .await
        .unwrap();

    daily_and_pet_villa(app, pool, id, key).await;
}

async fn daily_and_pet_villa(app: &Router, pool: &MySqlPool, id: u64, key: &str) {
    // DailyReward: getDailyData, getDailyTokenData, claimDailyXP, claimScrollOfWisdom, getAttendances, claimAttendanceReward
    let daily_data = amf_call(app, "DailyReward.getDailyData", json!([id, key])).await;
    assert_eq!(daily_data["status"], 1, "{daily_data}");
    assert_eq!(daily_data["tokens"], true);

    let daily_tok = amf_call(app, "DailyReward.getDailyTokenData", json!([id, key])).await;
    assert_eq!(daily_tok["status"], 1, "{daily_tok}");
    assert_eq!(
        amf_call(app, "DailyReward.getDailyTokenData", json!([id, key])).await["status"],
        2
    );

    let daily_xp = amf_call(app, "DailyReward.claimDailyXP", json!([id, key])).await;
    assert_eq!(daily_xp["status"], 1, "{daily_xp}");
    assert_eq!(
        amf_call(app, "DailyReward.claimDoubleXP", json!([id, key])).await["status"],
        2
    );

    let scroll = amf_call(app, "DailyReward.claimScrollOfWisdom", json!([id, key])).await;
    assert_eq!(scroll["status"], 1, "{scroll}");
    assert_eq!(
        amf_call(app, "DailyReward.claimScrollOfWisdom", json!([id, key])).await["status"],
        2
    );

    let att = amf_call(app, "DailyReward.getAttendances", json!([id, key])).await;
    assert_eq!(att["status"], 1, "{att}");
    assert!(att["count"].as_u64().unwrap_or(0) >= 1);
    if let Some(first_att) = att["items"].as_array().and_then(|a| a.first())
        && first_att["price"].as_u64().unwrap_or(99) <= 1
    {
        let att_claim = amf_call(
            app,
            "DailyReward.claimAttendanceReward",
            json!([id, key, first_att["id"]]),
        )
        .await;
        assert_eq!(att_claim["status"], 1, "{att_claim}");
    }

    // DailyRoulette: getData, spin
    let roul_data = amf_call(app, "DailyRoulette.getData", json!([id, key])).await;
    assert_eq!(roul_data["status"], 1, "{roul_data}");
    assert_eq!(roul_data["can_spin"], 1);
    let roul_spin = amf_call(app, "DailyRoulette.spin", json!([id, key])).await;
    assert_eq!(roul_spin["status"], 1, "{roul_spin}");

    // DailyScratch: getData, scratch
    let scr_data = amf_call(app, "DailyScratch.getData", json!([id, key])).await;
    assert_eq!(scr_data["status"], 1, "{scr_data}");
    assert!(scr_data["ticket"].as_i64().unwrap_or(0) >= 1);
    let scr_res = amf_call(app, "DailyScratch.scratch", json!([id, key])).await;
    assert_eq!(scr_res["status"], 1, "{scr_res}");

    // WelcomeLogin: get, claim
    let wel_data = amf_call(app, "WelcomeLogin.get", json!([id, key])).await;
    assert_eq!(wel_data["status"], 1, "{wel_data}");
    assert_eq!(wel_data["rewards"].as_array().unwrap().len(), 7);
    let wel_claim = amf_call(app, "WelcomeLogin.claim", json!([id, key, 0])).await;
    assert_eq!(wel_claim["status"], 1, "{wel_claim}");

    // Character AccountService: convertTokensToGold, renameCharacter
    let conv = amf_call(
        app,
        "CharacterService.convertTokensToGold",
        json!([key, id, 2]),
    )
    .await;
    assert_eq!(conv["status"], 1, "{conv}");

    sqlx::query("INSERT INTO character_items (character_id,item_id,category,quantity) VALUES (?,'essential_01','essential',5)")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    let new_char_name = format!("ren_{}", authentication::random_hex().unwrap());
    let ren = amf_call(
        app,
        "CharacterService.renameCharacter",
        json!([id, key, new_char_name]),
    )
    .await;
    assert_eq!(ren["status"], 1, "{ren}");

    // Pet Villa & PetCombination
    sqlx::query("INSERT INTO character_pets (character_id,pet_id,name,level,xp,maturity_points,created_at,updated_at) VALUES (?,'pet_regression','VillaPet1',30,0,120,NOW(),NOW()),(?,'pet_regression','VillaPet2',30,0,120,NOW(),NOW())")
        .bind(id)
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    let villa_data = amf_call(
        app,
        "PetService.executeService",
        json!(["getVillaData", [id, key]]),
    )
    .await;
    assert_eq!(villa_data["status"], 1, "{villa_data}");
    assert_eq!(villa_data["slots"].as_array().unwrap().len(), 4);
    let villa_pets = villa_data["pets"].as_array().unwrap();
    let p1 = villa_pets[villa_pets.len() - 2]["pet_id"].as_u64().unwrap();
    let p2 = villa_pets[villa_pets.len() - 1]["pet_id"].as_u64().unwrap();

    let train = amf_call(
        app,
        "PetService.executeService",
        json!(["trainPet", [id, key, p1]]),
    )
    .await;
    assert_eq!(train["status"], 1, "{train}");
    let skip = amf_call(
        app,
        "PetService.executeService",
        json!(["skipTraining", [id, key, p1]]),
    )
    .await;
    assert_eq!(skip["status"], 1, "{skip}");
    let checkout = amf_call(
        app,
        "PetService.executeService",
        json!(["checkoutPet", [id, key, p1]]),
    )
    .await;
    assert_eq!(checkout["status"], 1, "{checkout}");

    let unlock = amf_call(
        app,
        "PetService.executeService",
        json!(["unlockSlots", [id, key, "tokens"]]),
    )
    .await;
    assert_eq!(unlock["status"], 1, "{unlock}");

    let combo_data = amf_call(app, "PetCombination.getData", json!([id, key])).await;
    assert_eq!(combo_data["status"], 1, "{combo_data}");
    assert!(combo_data["boost"].is_null());

    let combo_boost = amf_call(app, "PetCombination.boostRate", json!([id, key])).await;
    assert_eq!(combo_boost["status"], 1, "{combo_boost}");

    let combo_res = amf_call(app, "PetCombination.combinePet", json!([id, key, p1, p2])).await;
    assert_eq!(combo_res["status"], 1, "{combo_res}");
    sqlx::query("DELETE FROM character_pets WHERE character_id=? AND id >= ?")
        .bind(id)
        .bind(p1)
        .execute(pool)
        .await
        .unwrap();

    social_and_shadow_war(app, pool, id, key).await;
}

async fn social_and_shadow_war(app: &Router, pool: &MySqlPool, id: u64, key: &str) {
    // Create a second temporary character for friend/shadow-war tests
    let uid: u64 = sqlx::query_scalar("SELECT user_id FROM characters WHERE id=?")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap();
    let res = sqlx::query(
        "INSERT INTO characters (user_id,name,level,`rank`,gold,created_at,updated_at) VALUES (?,'FriendTarget',85,7,5000,NOW(),NOW())",
    )
    .bind(uid)
    .execute(pool)
    .await
    .unwrap();
    let friend_id = res.last_insert_id();

    // FriendService: addFriend, friendRequests, acceptFriend, friends, setFavorite, getFavorite, search, recruitFriend, startBerantem, endBerantem, getItems, buyItem, removeFriend
    let add_req = amf_call(app, "FriendService.addFriend", json!([id, key, friend_id])).await;
    assert_eq!(add_req["status"], 1, "{add_req}");

    let reqs = amf_call(
        app,
        "FriendService.friendRequests",
        json!([friend_id, key, 1]),
    )
    .await;
    assert_eq!(reqs["status"], 1, "{reqs}");
    assert_eq!(reqs["Request"].as_array().unwrap().len(), 1);

    let acc = amf_call(
        app,
        "FriendService.acceptFriend",
        json!([friend_id, key, id]),
    )
    .await;
    assert_eq!(acc["status"], 1, "{acc}");

    let flist = amf_call(app, "FriendService.friends", json!([id, key, 1])).await;
    assert_eq!(flist["status"], 1, "{flist}");
    assert_eq!(flist["friends"].as_array().unwrap().len(), 1);

    let fav = amf_call(
        app,
        "FriendService.setFavorite",
        json!([id, key, friend_id]),
    )
    .await;
    assert_eq!(fav["status"], 1, "{fav}");
    let fav_list = amf_call(app, "FriendService.getFavorite", json!([id, key, 1])).await;
    assert_eq!(fav_list["friends"].as_array().unwrap().len(), 1);

    let searched = amf_call(
        app,
        "FriendService.search",
        json!([id, key, "FriendTarget"]),
    )
    .await;
    assert_eq!(searched["friends"].as_array().unwrap().len(), 1);

    let rec_friend = amf_call(
        app,
        "FriendService.recruitFriend",
        json!([id, key, friend_id]),
    )
    .await;
    assert_eq!(rec_friend["status"], 1, "{rec_friend}");
    let room = amf_call(app, "CharacterService.getMissionRoomData", json!([id, key])).await;
    assert_eq!(room["status"], 1, "{room}");
    assert!(
        room["recruit"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["type"] == "char")
    );

    let spar_hash = authentication::sha256_hex(&format!("{id}{friend_id}{key}"));
    let spar_start = amf_call(
        app,
        "FriendService.startBerantem",
        json!([id, friend_id, spar_hash, key]),
    )
    .await;
    assert_eq!(spar_start["status"], 1, "{spar_start}");
    let spar_code = spar_start["code"].as_str().unwrap();
    let end_hash = authentication::sha256_hex(&format!("{id}{spar_code}{key}"));
    let spar_end = amf_call(
        app,
        "FriendService.endBerantem",
        json!([id, spar_code, end_hash, key, null]),
    )
    .await;
    assert_eq!(spar_end["status"], 1, "{spar_end}");

    let f_shop = amf_call(app, "FriendService.getItems", json!([id, key])).await;
    assert_eq!(f_shop["status"], 1, "{f_shop}");
    sqlx::query(
        "UPDATE character_items SET quantity=100 WHERE character_id=? AND item_id='material_1002'",
    )
    .bind(id)
    .execute(pool)
    .await
    .unwrap();
    if let Some(first_shop) = f_shop["items"].as_array().and_then(|i| i.first()) {
        let buy_f = amf_call(
            app,
            "FriendService.buyItem",
            json!([id, key, first_shop["id"]]),
        )
        .await;
        assert_eq!(buy_f["status"], 1, "{buy_f}");
    }

    let rem_f = amf_call(
        app,
        "FriendService.removeFriend",
        json!([id, key, friend_id]),
    )
    .await;
    assert_eq!(rem_f["status"], 1, "{rem_f}");

    // MailService: getMails, openMail, claimReward, claimAllRewards, deleteMail
    sqlx::query(
        "INSERT INTO character_mails (character_id,title,sender,body,`type`,rewards,claimed,viewed,created_at,updated_at) VALUES (?,'Welcome Gift','Admin','Enjoy!','system','[\"gold_500\",\"tokens_10\"]',0,0,NOW(),NOW())",
    )
    .bind(id)
    .execute(pool)
    .await
    .unwrap();
    let mails = amf_call(app, "Mail.executeService", json!(["getMails", [id, key]])).await;
    assert_eq!(mails["status"], 1, "{mails}");
    let mail_id = mails["mails"][0]["id"].as_u64().unwrap();
    let opened = amf_call(
        app,
        "Mail.executeService",
        json!(["openMail", [id, key, mail_id]]),
    )
    .await;
    assert_eq!(opened["status"], 1, "{opened}");
    let claimed = amf_call(
        app,
        "Mail.executeService",
        json!(["claimReward", [id, key, mail_id]]),
    )
    .await;
    assert_eq!(claimed["status"], 1, "{claimed}");
    let deleted = amf_call(
        app,
        "Mail.executeService",
        json!(["deleteMail", [id, key, mail_id]]),
    )
    .await;
    assert_eq!(deleted["status"], 1, "{deleted}");

    // Missing/incorrect sessions must never mint or rotate clan/crew credentials.
    for service in ["ClanService", "CrewService"] {
        for args in [json!([id]), json!([id, ""]), json!([id, "wrong_session"])] {
            let denied = amf_call(app, &format!("{service}.login"), args).await;
            assert_eq!(denied["status"], 0, "{denied}");
        }
    }
    // ClanService & CrewService bridge
    let clan_status = amf_call(app, "ClanService.getCharacterClan", json!([id, key])).await;
    assert_eq!(clan_status["status"], 1, "{clan_status}");
    let crew_status = amf_call(app, "CrewService.getCharacterCrew", json!([id, key])).await;
    assert_eq!(crew_status["status"], 1, "{crew_status}");
    let saved_token: String =
        sqlx::query_scalar("SELECT token FROM clan_auth_tokens WHERE character_id=?")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(
        amf_call(app, "ClanService.login", json!([id, ""])).await["status"],
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT token FROM clan_auth_tokens WHERE character_id=?")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap(),
        saved_token
    );

    // ShadowWar: getSeason, getStatus, getPresets, savePreset, usePreset, getEnemies, startBattle, finishBattle, globalLeaderboard, getProfile
    sqlx::query("DELETE FROM shadow_war_seasons WHERE num=999")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO shadow_war_seasons (num, start_at, end_at, active, created_at, updated_at) VALUES (999, DATE_SUB(NOW(), INTERVAL 1 HOUR), DATE_ADD(NOW(), INTERVAL 2 HOUR), 1, NOW(), NOW())",
    )
    .execute(pool)
    .await
    .unwrap();

    let sw_season = amf_call(
        app,
        "ShadowWar.executeService",
        json!(["getSeason", [id, key]]),
    )
    .await;
    assert_eq!(sw_season["status"], 1, "{sw_season}");
    assert_eq!(sw_season["active"], true);

    let sw_status = amf_call(
        app,
        "ShadowWar.executeService",
        json!(["getStatus", [id, key]]),
    )
    .await;
    assert_eq!(sw_status["status"], 1, "{sw_status}");
    assert_eq!(sw_status["energy"], 100);

    let sw_presets = amf_call(
        app,
        "ShadowWar.executeService",
        json!(["getPresets", [id, key]]),
    )
    .await;
    assert_eq!(sw_presets["status"], 1, "{sw_presets}");
    let sw_use = amf_call(
        app,
        "ShadowWar.executeService",
        json!(["usePreset", [id, key, 2]]),
    )
    .await;
    assert_eq!(sw_use["active"], 2);

    let sw_enemies = amf_call(
        app,
        "ShadowWar.executeService",
        json!(["getEnemies", [id, key]]),
    )
    .await;
    assert_eq!(sw_enemies["status"], 1, "{sw_enemies}");

    let sw_start = amf_call(
        app,
        "ShadowWar.executeService",
        json!(["startBattle", [id, key, friend_id]]),
    )
    .await;
    assert_eq!(sw_start["status"], 1, "{sw_start}");
    let sw_code = sw_start["id"].as_str().unwrap();
    let sw_finish = amf_call(
        app,
        "ShadowWar.executeService",
        json!(["finishBattle", [id, key, sw_code, 1500, "e30=", "hash", 1]]),
    )
    .await;
    assert_eq!(sw_finish["status"], 1, "{sw_finish}");
    assert!(sw_finish["trophy"].as_i64().unwrap_or(0) > 0);

    let sw_lb = amf_call(
        app,
        "ShadowWar.executeService",
        json!(["globalLeaderboard", [id, key]]),
    )
    .await;
    assert_eq!(sw_lb["status"], 1, "{sw_lb}");
    assert!(!sw_lb["leaderboard"].as_array().unwrap().is_empty());

    let sw_prof = amf_call(
        app,
        "ShadowWar.executeService",
        json!(["getProfile", [id, key, id]]),
    )
    .await;
    assert_eq!(sw_prof["status"], 1, "{sw_prof}");

    sqlx::query("DELETE FROM shadow_war_seasons WHERE num=999")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM characters WHERE id=?")
        .bind(friend_id)
        .execute(pool)
        .await
        .unwrap();

    events_seasonal_pvp_and_http(app, pool, id, key).await;
}

async fn http_post_json(
    app: &Router,
    uri: &str,
    form_body: &str,
) -> (axum::http::StatusCode, serde_json::Value) {
    use tower::ServiceExt;
    let response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(uri)
                .header(
                    axum::http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body(axum::body::Body::from(form_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let val = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, val)
}

async fn events_seasonal_pvp_and_http(app: &Router, pool: &MySqlPool, id: u64, key: &str) {
    let uid: u64 = sqlx::query_scalar("SELECT user_id FROM characters WHERE id=?")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE users SET tokens=50000 WHERE id=?")
        .bind(uid)
        .execute(pool)
        .await
        .unwrap();

    // Events.executeService ("get")
    let ev_get = amf_call(
        app,
        "EventsService.executeService",
        json!(["get", [id, key]]),
    )
    .await;
    assert_eq!(ev_get["status"], 1, "{ev_get}");

    // Seed a test MonsterHunter & HalloweenMenu event in game_events
    sqlx::query("DELETE FROM game_events WHERE panel IN ('MonsterHunter','HalloweenMenu')")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO game_events (title,`type`,panel,image_url,active,`data`,created_at,updated_at) VALUES \
         ('Monster Hunter','seasonal','MonsterHunter','mh.swf',1,'{\"energy_max\":100,\"energy_cost\":20,\"boss_id\":\"ene_01\",\"rewards\":{\"xp\":100,\"gold\":200}}',NOW(),NOW()), \
         ('Halloween 2025','seasonal','HalloweenMenu','hw.swf',1,'{\"energy_max\":8,\"bosses\":[\"ene_01\"],\"rewards\":{\"xp\":100,\"gold\":200},\"milestones\":[{\"requirement\":1,\"id\":\"gold_500\"}]}',NOW(),NOW())",
    )
    .execute(pool)
    .await
    .unwrap();

    let ev_panel = amf_call(
        app,
        "EventsService.executeService",
        json!(["MonsterHunter", [id, key]]),
    )
    .await;
    assert_eq!(ev_panel["status"], 1, "{ev_panel}");
    assert_eq!(ev_panel["boss_id"], "ene_01");

    // MonsterHunterEvent2023 flow
    let mh_data = amf_call(app, "MonsterHunterEvent2023.getEventData", json!([id, key])).await;
    assert_eq!(mh_data["status"], 1, "{mh_data}");
    assert_eq!(mh_data["energy"], 100);
    let mh_start = amf_call(
        app,
        "MonsterHunterEvent2023.startBattle",
        json!([id, "ene_01", 10, "", "h", key]),
    )
    .await;
    assert_eq!(mh_start["status"], 1, "{mh_start}");
    let mh_code = mh_start["code"].as_str().unwrap();
    let mh_fin = amf_call(
        app,
        "MonsterHunterEvent2023.finishBattle",
        json!([id, "ene_01", mh_code, 500, "", 1, "h", key]),
    )
    .await;
    assert_eq!(mh_fin["status"], 1, "{mh_fin}");

    // HalloweenEvent2025 flow
    let hw_data = amf_call(app, "HalloweenEvent2025.getBattleData", json!([id, key])).await;
    assert_eq!(hw_data["status"], 1, "{hw_data}");
    let hw_start = amf_call(
        app,
        "HalloweenEvent2025.startBattle",
        json!([id, 0, "h", key]),
    )
    .await;
    assert_eq!(hw_start["status"], 1, "{hw_start}");
    let hw_code = hw_start["code"].as_str().unwrap();
    let hw_fin = amf_call(
        app,
        "HalloweenEvent2025.finishBattle",
        json!([id, "ene_01", hw_code, 500, "", 1, "h", key]),
    )
    .await;
    assert_eq!(hw_fin["status"], 1, "{hw_fin}");
    let hw_claim = amf_call(
        app,
        "HalloweenEvent2025.claimBonusRewards",
        json!([id, key, 0]),
    )
    .await;
    assert_eq!(hw_claim["status"], 1, "{hw_claim}");

    // Leaderboard.getData
    let lb = amf_call(
        app,
        "Leaderboard.getData",
        json!([id, key, "halloween_2025"]),
    )
    .await;
    assert_eq!(lb["status"], 1, "{lb}");
    assert!(!lb["data"].as_array().unwrap().is_empty());

    // PackageEvent.buyChuninPackage
    let pkg_buy = amf_call(
        app,
        "PackageEvent.executeService",
        json!(["buyChuninPackage", [id, key]]),
    )
    .await;
    assert_eq!(pkg_buy["status"], 1, "{pkg_buy}");

    // Giveaway flow
    let gw_res = sqlx::query(
        "INSERT INTO giveaways (title, description, prizes, requirements, ends_at, created_at, updated_at) \
         VALUES ('Test Giveaway', 'Win Gold!', '[\"gold_1000\"]', '[{\"type\":\"level\",\"value\":1}]', DATE_ADD(NOW(), INTERVAL 1 DAY), NOW(), NOW())",
    )
    .execute(pool)
    .await
    .unwrap();
    let gw_id = gw_res.last_insert_id();
    let gw_list = amf_call(app, "Giveaway.get", json!([id, key])).await;
    assert_eq!(gw_list["status"], 1, "{gw_list}");
    let gw_join = amf_call(app, "Giveaway.participate", json!([id, key, gw_id])).await;
    assert_eq!(gw_join["status"], 1, "{gw_join}");
    let gw_hist = amf_call(app, "Giveaway.history", json!([id, key])).await;
    assert_eq!(gw_hist["status"], 1, "{gw_hist}");
    sqlx::query("DELETE FROM giveaways WHERE id=?")
        .bind(gw_id)
        .execute(pool)
        .await
        .unwrap();

    // PvP AMF service flow
    let pvp_acc = amf_call(app, "PvP.checkAccess", json!([id, key])).await;
    assert_eq!(pvp_acc["status"], 1, "{pvp_acc}");
    let pvp_stats = amf_call(app, "PvP.getCharacterStats", json!([id, key])).await;
    assert_eq!(pvp_stats["status"], 1, "{pvp_stats}");
    let pvp_lb = amf_call(app, "PvP.getLeaderboard", json!([id, key, "trophy"])).await;
    assert_eq!(pvp_lb["status"], 1, "{pvp_lb}");
    let pvp_act = amf_call(app, "PvP.getBattleActivity", json!([id, key])).await;
    assert_eq!(pvp_act["status"], 1, "{pvp_act}");
    let pvp_bug = amf_call(
        app,
        "PvP.reportBug",
        json!([id, key, "Test Bug", "Bug description"]),
    )
    .await;
    assert_eq!(pvp_bug["status"], 1, "{pvp_bug}");

    // Clan & Crew HTTP REST endpoints
    let clan_amf = amf_call(app, "ClanService.getCharacterClan", json!([id, key])).await;
    let clan_token = clan_amf["clan_token"].as_str().unwrap();
    let (st, clan_season) =
        http_post_json(app, "/clan/season", &format!("token={clan_token}")).await;
    assert_eq!(st, axum::http::StatusCode::OK);
    assert!(clan_season["season"]["id"].as_u64().is_some());

    let clan_name = format!("Clan_{}", &authentication::random_hex().unwrap()[..8]);
    let (st, created_clan) = http_post_json(
        app,
        "/clan/create",
        &format!("token={clan_token}&name={clan_name}"),
    )
    .await;
    assert_eq!(st, axum::http::StatusCode::OK);
    assert_eq!(created_clan, json!("ok"));

    let (st, p_clan) =
        http_post_json(app, "/clan/player/clan", &format!("token={clan_token}")).await;
    assert_eq!(st, axum::http::StatusCode::OK);
    assert_eq!(p_clan["clan"]["name"], clan_name);

    let crew_amf = amf_call(app, "CrewService.getCharacterCrew", json!([id, key])).await;
    let crew_token = crew_amf["crew_token"].as_str().unwrap();
    let crew_name = format!("Crew_{}", &authentication::random_hex().unwrap()[..8]);
    let (st, created_crew) = http_post_json(
        app,
        "/crew/create",
        &format!("token={crew_token}&name={crew_name}"),
    )
    .await;
    assert_eq!(st, axum::http::StatusCode::OK);
    assert_eq!(created_crew["status"], "ok");

    let (st, castles) =
        http_post_json(app, "/crew/battle/castles", &format!("token={crew_token}")).await;
    assert_eq!(st, axum::http::StatusCode::OK);
    assert_eq!(castles["castles"].as_array().unwrap().len(), 7);

    sqlx::query("DELETE FROM clans WHERE master_id=?")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM crews WHERE master_id=?")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM game_events WHERE panel IN ('MonsterHunter','HalloweenMenu')")
        .execute(pool)
        .await
        .unwrap();

    admin_dashboard_endpoints(app, pool, id).await;

    sqlx::query("UPDATE character_pets SET xp=20 WHERE character_id=?")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
}

async fn http_admin_json(
    app: &Router,
    method: &str,
    uri: &str,
    secret: &str,
    body: Option<serde_json::Value>,
) -> (axum::http::StatusCode, serde_json::Value) {
    use tower::ServiceExt;
    let payload = body.map(|b| b.to_string()).unwrap_or_default();
    let response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method(method)
                .uri(uri)
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .header(
                    axum::http::header::AUTHORIZATION,
                    format!("Bearer {secret}"),
                )
                .body(axum::body::Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let val = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, val)
}

async fn admin_dashboard_endpoints(app: &Router, pool: &MySqlPool, id: u64) {
    let user=sqlx::query("SELECT u.id,u.username,u.password,u.account_type FROM users u JOIN characters c ON c.user_id=u.id WHERE c.id=?").bind(id).fetch_one(pool).await.unwrap();
    let uid = user.get::<u64, _>("id");
    let original_type = user.get::<i8, _>("account_type");
    // A normal account must not gain admin access through a login.
    let (st, _) = http_admin_json(
        app,
        "POST",
        "/api/admin/login",
        "",
        Some(json!({"username":user.get::<String,_>("username"),"password":"NinjaTest123!"})),
    )
    .await;
    assert_eq!(st, axum::http::StatusCode::FORBIDDEN);
    sqlx::query("UPDATE users SET account_type=99 WHERE id=?")
        .bind(uid)
        .execute(pool)
        .await
        .unwrap();
    // Supplying the stored password hash as a password must also fail.
    let (st,_)=http_admin_json(app,"POST","/api/admin/login","",Some(json!({"username":user.get::<String,_>("username"),"password":user.get::<String,_>("password")}))).await;
    assert_eq!(st, axum::http::StatusCode::UNAUTHORIZED);
    let (st, login_res) = http_admin_json(
        app,
        "POST",
        "/api/admin/login",
        "",
        Some(json!({"username":user.get::<String,_>("username"),"password":"NinjaTest123!"})),
    )
    .await;
    assert_eq!(st, axum::http::StatusCode::OK, "{login_res}");
    let secret = login_res["token"].as_str().unwrap();

    // Unauthorized without secret
    let (st, _) = http_admin_json(app, "GET", "/api/admin/overview", "wrong_secret", None).await;
    assert_eq!(st, axum::http::StatusCode::UNAUTHORIZED);

    // A guessed secret must not log in.
    let (st, _) = http_admin_json(
        app,
        "POST",
        "/api/admin/login",
        "",
        Some(json!({ "secret": "guessed_admin_secret" })),
    )
    .await;
    assert_eq!(st, axum::http::StatusCode::BAD_REQUEST);

    // Overview
    let (st, ov) = http_admin_json(app, "GET", "/api/admin/overview", secret, None).await;
    assert_eq!(st, axum::http::StatusCode::OK);
    assert!(ov["counts"]["characters"].as_i64().unwrap_or(0) >= 1);

    // List character resource
    let (st, chars) = http_admin_json(
        app,
        "GET",
        "/api/admin/resources/characters?limit=5",
        secret,
        None,
    )
    .await;
    assert_eq!(st, axum::http::StatusCode::OK);
    assert!(!chars["data"].as_array().unwrap().is_empty());

    // Execute AdminCommandService add_gold
    let (st, cmd) = http_admin_json(
        app,
        "POST",
        "/api/admin/commands/execute",
        secret,
        Some(json!({
            "character_id": id,
            "command_type": "add_gold",
            "params": { "amount": 1000 }
        })),
    )
    .await;
    assert_eq!(st, axum::http::StatusCode::OK);
    assert_eq!(cmd["success"], true);

    // Put & Get config
    let (st, _) = http_admin_json(
        app,
        "PUT",
        "/api/admin/configs/game_settings",
        secret,
        Some(json!({ "value": { "special_deals_visible": true } })),
    )
    .await;
    assert_eq!(st, axum::http::StatusCode::OK);
    let (st, cfg_get) =
        http_admin_json(app, "GET", "/api/admin/configs/game_settings", secret, None).await;
    assert_eq!(st, axum::http::StatusCode::OK);
    assert_eq!(cfg_get["value"]["special_deals_visible"], true);

    // World chat announce & list
    let (st, ann) = http_admin_json(
        app,
        "POST",
        "/api/admin/chat/announce",
        secret,
        Some(json!({ "channel": "global", "message": "Welcome ninjas!" })),
    )
    .await;
    assert_eq!(st, axum::http::StatusCode::OK);
    assert_eq!(ann["ok"], true);
    sqlx::query("UPDATE users SET account_type=? WHERE id=?")
        .bind(original_type)
        .bind(uid)
        .execute(pool)
        .await
        .unwrap();
}
