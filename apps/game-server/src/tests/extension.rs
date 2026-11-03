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
    sqlx::query("INSERT INTO character_items (character_id,item_id,category,quantity) VALUES (?,'essential_02','essential',5)").bind(id).execute(pool).await.unwrap();
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
}
