use super::amf_call;
use axum::Router;
use persistence::{MySqlPool, sqlx};
use serde_json::json;

pub async fn run(app: &Router, pool: &MySqlPool, id: u64, key: &str) {
    talent_client_numbers(app, pool, id, key).await;
    original_event_contract(app, pool, id, key).await;
    let tokens: i32 = sqlx::query_scalar(
        "SELECT u.tokens FROM users u JOIN characters c ON c.user_id=u.id WHERE c.id=?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(
        amf_call(app, "CharacterService.getGearPresets", json!([id, key])).await["presets"],
        json!([])
    );
    assert_eq!(
        amf_call(
            app,
            "CharacterService.unlockGearPresetSlot",
            json!([id, key])
        )
        .await["tokens"],
        tokens
    );
    let list = amf_call(app, "CharacterService.getGearPresets", json!([id, key])).await;
    let preset = list["presets"][0]["id"].clone();
    assert!(preset.as_u64().is_some(), "{list}");
    assert_eq!(
        amf_call(
            app,
            "CharacterService.renameGearPreset",
            json!([id, key, preset, "Battle"])
        )
        .await["status"],
        1
    );
    assert_eq!(
        amf_call(
            app,
            "CharacterService.updateGearPreset",
            json!([id, key, preset, "wpn_not_owned", "", "", "", "", ""])
        )
        .await["status"],
        2
    );
    assert_ne!(
        amf_call(
            app,
            "CharacterService.renameGearPreset",
            json!([id, "wrong", preset, "Hacked"])
        )
        .await["status"],
        1
    );
    for _ in 0..3 {
        assert_eq!(
            amf_call(
                app,
                "CharacterService.unlockGearPresetSlot",
                json!([id, key])
            )
            .await["status"],
            1
        );
    }
    assert_eq!(
        amf_call(
            app,
            "CharacterService.unlockGearPresetSlot",
            json!([id, key])
        )
        .await["status"],
        2
    );
    let list = amf_call(app, "CharacterService.getGearPresets", json!([id, key])).await;
    assert_eq!(list["presets"].as_array().unwrap().len(), 4);
    assert_eq!(list["presets"][0]["name"], "Battle");
    let after: i32 = sqlx::query_scalar(
        "SELECT u.tokens FROM users u JOIN characters c ON c.user_id=u.id WHERE c.id=?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(after, tokens - 1500);
    sqlx::query("UPDATE characters SET element_1=1,element_2=0,element_3=0 WHERE id=?")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM character_items WHERE character_id=? AND item_id='essential_10'")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    crate::inventory::give(&mut tx, id as i64, "essential_10", 1)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        amf_call(app, "ScrollOfWisdom.getData", json!([id, key])).await["scrolls"],
        1
    );
    assert_eq!(
        amf_call(app, "ScrollOfWisdom.claimSkill", json!([id, key, 2])).await["status"],
        2
    );
    let learned = amf_call(app, "ScrollOfWisdom.claimSkill", json!([id, key, 1])).await;
    assert_eq!(learned["status"], 1, "{learned}");
    assert!(
        learned["skills"]
            .as_str()
            .unwrap()
            .split(',')
            .any(|s| s == "skill_329")
    );
    assert_eq!(
        amf_call(app, "ScrollOfWisdom.getData", json!([id, key])).await["scrolls"],
        0
    );
    assert_eq!(
        amf_call(app, "ScrollOfWisdom.claimSkill", json!([id, key, 1])).await["status"],
        2
    );
    let loaded = amf_call(app, "SystemLogin.getCharacterData", json!([id, key])).await;
    assert!(
        loaded["character_inventory"]["char_skills"]
            .as_str()
            .unwrap()
            .contains("skill_329")
    );
}

async fn original_event_contract(app: &Router, pool: &MySqlPool, id: u64, key: &str) {
    let catalog: serde_json::Value =
        serde_json::from_str(include_str!("../../../../database/catalogs/events.json")).unwrap();
    let cfg = catalog
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["panel"] == "ChristmasMenu")
        .unwrap()["data"]
        .clone();
    sqlx::query("DELETE FROM game_events WHERE panel='ChristmasMenu'")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO game_events(title,`type`,panel,image_url,active,`data`) VALUES('Regression Christmas','seasonal','ChristmasMenu','',1,?)").bind(cfg).execute(pool).await.unwrap();
    let start_hash = authentication::sha256_hex(&format!("{id}ene_2117stats10"));
    let battle = amf_call(
        app,
        "ChristmasEvent2025.startBattle",
        json!([id, "ene_2117", 10, "stats", start_hash, key]),
    )
    .await;
    assert_eq!(battle["status"], 1, "{battle}");
    let code = battle["code"].as_str().unwrap();
    assert_eq!(
        battle["hash"],
        authentication::sha256_hex(&format!("ene_2117{code}{id}"))
    );
    let finish_hash = authentication::sha256_hex(&format!("{id}ene_2117{code}5001"));
    let rejected = amf_call(
        app,
        "ChristmasEvent2025.finishBattle",
        json!([id, "ene_2117", "bad-code", 500, finish_hash, 1, key]),
    )
    .await;
    assert_eq!(rejected["status"], 2);
    let finished = amf_call(
        app,
        "ChristmasEvent2025.finishBattle",
        json!([id, "ene_2117", code, 500, finish_hash, 1, key]),
    )
    .await;
    assert_eq!(finished["status"], 1, "{finished}");
    assert!(finished["result"][0].as_i64().unwrap() > 0);
    assert!(
        finished["result"][2]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "material_2231")
    );
    assert_eq!(
        amf_call(
            app,
            "ChristmasEvent2025.finishBattle",
            json!([id, "ene_2117", code, 500, finish_hash, 1, key])
        )
        .await["status"],
        2
    );
    let mini = amf_call(app, "ChristmasEvent2025.startMinigame", json!([id, key])).await;
    assert_eq!(mini["status"], 1, "{mini}");
    let mini_code = mini["code"].as_str().unwrap();
    let mini_hash = authentication::sha256_hex(&format!("{id}|{key}|100|{mini_code}"));
    assert_eq!(
        amf_call(
            app,
            "ChristmasEvent2025.finishMinigame",
            json!([id, key, 100, "wrong"])
        )
        .await["status"],
        2
    );
    let mini_finish = amf_call(
        app,
        "ChristmasEvent2025.finishMinigame",
        json!([id, key, 100, mini_hash]),
    )
    .await;
    assert_eq!(mini_finish["status"], 1, "{mini_finish}");
    assert_eq!(mini_finish["current_energy"], 7);
    assert_eq!(
        mini_finish["rewards"],
        json!(["material_2230", "material_2231"])
    );
    assert_eq!(
        amf_call(
            app,
            "ChristmasEvent2025.finishMinigame",
            json!([id, key, 100, mini_hash])
        )
        .await["status"],
        2
    );
    let gacha_data = amf_call(app, "ChristmasEvent2025.getGachaData", json!([id, key])).await;
    assert!(gacha_data["coin"].as_i64().unwrap() >= 2);
    let before: i32 = sqlx::query_scalar(
        "SELECT u.tokens FROM users u JOIN characters c ON c.user_id=u.id WHERE c.id=?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap();
    let draw = amf_call(
        app,
        "ChristmasEvent2025.getGachaRewards",
        json!([id, key, "tokens", 3]),
    )
    .await;
    assert_eq!(draw["status"], 1, "{draw}");
    assert_eq!(draw["rewards"].as_array().unwrap().len(), 3);
    assert!(draw["coin"].as_i64().is_some());
    let token_rewards: i32 = draw["rewards"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str()?.strip_prefix("tokens_")?.parse::<i32>().ok())
        .sum();
    let after: i32 = sqlx::query_scalar(
        "SELECT u.tokens FROM users u JOIN characters c ON c.user_id=u.id WHERE c.id=?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(after, before - 50 + token_rewards);
    let gacha_bonus = amf_call(
        app,
        "ChristmasEvent2025.getBonusGachaRewards",
        json!([id, key]),
    )
    .await;
    assert_eq!(gacha_bonus["total_spins"], 3);
    assert!(gacha_bonus["data"].is_array());
    let bonus = amf_call(app, "ChristmasEvent2025.getBonusRewards", json!([id, key])).await;
    assert_eq!(bonus["milestone"], 1);
    assert_eq!(bonus["rewards"].as_array().unwrap().len(), 8);
    assert_eq!(
        amf_call(
            app,
            "ChristmasEvent2025.claimBonusRewards",
            json!([id, key, 0])
        )
        .await["status"],
        2
    );
    sqlx::query("DELETE FROM game_events WHERE panel='ChristmasMenu'")
        .execute(pool)
        .await
        .unwrap();
}

async fn talent_client_numbers(app: &Router, pool: &MySqlPool, id: u64, key: &str) {
    // This regression supplies both secret talents even on an older minimal schema.
    let mut shop = crate::talents::config(pool, "talent_shop").await.unwrap();
    if !shop["secret"]
        .as_array()
        .is_some_and(|a| a.iter().any(|v| v["id"] == "wood"))
    {
        shop["secret"] = json!([{"id":"wood"}]);
        sqlx::query("UPDATE game_configs SET value=? WHERE `key`='talent_shop'")
            .bind(shop)
            .execute(pool)
            .await
            .unwrap();
    }
    let mut info = crate::talents::config(pool, "talent_info").await.unwrap();
    if info.get("wood").is_none() {
        info["wood"] = json!({"is_emblem":false,"price_gold":1000000,"price_token":0});
        sqlx::query("UPDATE game_configs SET value=? WHERE `key`='talent_info'")
            .bind(info)
            .execute(pool)
            .await
            .unwrap();
    }
    sqlx::query("UPDATE characters SET talent_1='de',talent_2='sound',talent_3='',tp=2540,`rank`=5 WHERE id=?").bind(id).execute(pool).await.unwrap();
    sqlx::query("DELETE FROM character_talents WHERE character_id=? AND skill_id='skill_1006'")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    // ActionScript Number is a floating-point AMF value, unlike the old bool fixtures.
    let upgraded = amf_call(
        app,
        "TalentService.upgradeSkill",
        json!([id, key, "skill_1006", 1.0]),
    )
    .await;
    assert_eq!(upgraded["status"], 1, "{upgraded}");
    assert_eq!(upgraded["current_tp"], 0);
    let level: i32 = sqlx::query_scalar(
        "SELECT level FROM character_talents WHERE character_id=? AND skill_id='skill_1006'",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(level, 10);
    let locked = amf_call(
        app,
        "TalentService.discoverTalent",
        json!([id, key, "Secret", "wood"]),
    )
    .await;
    assert_eq!(locked["status"], 2, "{locked}");
    sqlx::query("UPDATE characters SET `rank`=7,gold=20000000 WHERE id=?")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    let second = amf_call(
        app,
        "TalentService.discoverTalent",
        json!([id, key, "Secret", "wood"]),
    )
    .await;
    assert_eq!(second["status"], 1, "{second}");
    assert_eq!(second["newt"], 3);
    sqlx::query(
        "UPDATE characters SET talent_1=NULL,talent_2=NULL,talent_3=NULL,`rank`=9 WHERE id=?",
    )
    .bind(id)
    .execute(pool)
    .await
    .unwrap();
}
