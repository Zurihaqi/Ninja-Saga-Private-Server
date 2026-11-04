use super::amf_call;
use axum::Router;
use persistence::{MySqlPool, sqlx};
use serde_json::json;

pub async fn run(app: &Router, pool: &MySqlPool, id: u64, key: &str, pet: u64) {
    let tokens_before: i32 = sqlx::query_scalar(
        "SELECT u.tokens FROM users u JOIN characters c ON c.user_id=u.id WHERE c.id=?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(
        amf_call(
            app,
            "CharacterService.useAnimation",
            json!([id, key, "ani_1"])
        )
        .await["status"],
        1
    );
    assert_eq!(
        amf_call(
            app,
            "CharacterService.useAnimation",
            json!([id, key, "ani_2"])
        )
        .await["status"],
        2
    );
    let buy = amf_call(
        app,
        "CharacterService.buyAnimation",
        json!([id, key, "ani_2"]),
    )
    .await;
    assert_eq!(buy["status"], 1, "{buy}");
    assert_eq!(buy["data"]["account_tokens"], tokens_before - 1000);
    assert_eq!(
        amf_call(
            app,
            "CharacterService.buyAnimation",
            json!([id, key, "ani_2"])
        )
        .await["status"],
        2
    );
    assert_eq!(
        amf_call(
            app,
            "CharacterService.useAnimation",
            json!([id, key, "ani_2"])
        )
        .await["status"],
        1
    );
    let loaded = amf_call(app, "SystemLogin.getCharacterData", json!([id, key])).await;
    assert!(
        loaded["character_inventory"]["char_animations"]
            .as_str()
            .unwrap()
            .split(',')
            .any(|a| a == "ani_2")
    );

    sqlx::query("INSERT INTO character_items (character_id,item_id,category,quantity) VALUES (?,'material_899','material',2),(?,'material_200','material',9)").bind(id).bind(id).execute(pool).await.unwrap();
    let s = amf_call(app, "BattleSystemService.getMissionSData", json!([id, key])).await;
    assert_eq!(s["status"], 1, "{s}");
    assert_eq!(s["materials"]["material_899"], 2);
    assert_eq!(s["max_energy"], 25);
    assert_eq!(
        amf_call(app, "BattleSystemService.getMissionSData", json!([id, ""])).await["status"],
        0
    );

    assert_eq!(
        amf_call(
            app,
            "PetService.executeService",
            json!(["feed", [id, key, pet, "item_01"]])
        )
        .await["status"],
        2
    );
    let fed = amf_call(
        app,
        "PetService.executeService",
        json!(["feed", [id, key, pet, "material_200"]]),
    )
    .await;
    assert_eq!(fed["mp"], 13, "{fed}");
    assert_eq!(
        amf_call(
            app,
            "PetService.executeService",
            json!(["feed", [id, key, pet + 999999, "material_200"]])
        )
        .await["status"],
        2
    );
    for _ in 0..7 {
        assert_eq!(
            amf_call(
                app,
                "PetService.executeService",
                json!(["feed", [id, key, pet, "material_200"]])
            )
            .await["status"],
            1
        );
    }
    assert_eq!(
        amf_call(
            app,
            "PetService.executeService",
            json!(["feed", [id, key, pet, "material_200"]])
        )
        .await["status"],
        2
    );
    assert_eq!(
        sqlx::query_scalar::<_, i32>(
            "SELECT quantity FROM character_items WHERE character_id=? AND item_id='material_200'"
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap(),
        1
    );
    let pets = amf_call(
        app,
        "PetService.executeService",
        json!(["getPets", [id, key]]),
    )
    .await;
    assert_eq!(pets["pets"][0]["pet_mp"], 100);
    assert_eq!(pets["pets"][0]["pet_skills"], "1,0,0,0,0,0");
    let config: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT c.skills FROM pets c JOIN character_pets p ON p.pet_id=c.pet_id WHERE p.id=?",
    )
    .bind(pet)
    .fetch_optional(pool)
    .await
    .unwrap()
    .flatten();
    let required = |slot: usize| {
        config
            .as_ref()
            .and_then(serde_json::Value::as_array)
            .and_then(|v| v.get(slot - 1))
            .and_then(|s| s["level"].as_i64())
            .unwrap_or(((slot - 1) * 10) as i64)
    };
    if required(2) > 1 {
        sqlx::query("UPDATE character_pets SET level=? WHERE id=?")
            .bind(required(2) - 1)
            .bind(pet)
            .execute(pool)
            .await
            .unwrap();
        assert_eq!(
            amf_call(
                app,
                "PetService.executeService",
                json!(["learnSkill", [id, key, pet, 2, "mc2"]])
            )
            .await["status"],
            2
        );
    }
    sqlx::query("UPDATE character_pets SET level=? WHERE id=?")
        .bind(required(2).max(required(3)))
        .bind(pet)
        .execute(pool)
        .await
        .unwrap();
    let request = json!(["learnSkill", [id, key, pet, 2, "mc2"]]);
    let (a, b) = tokio::join!(
        amf_call(app, "PetService.executeService", request.clone()),
        amf_call(app, "PetService.executeService", request)
    );
    assert_eq!(
        [&a, &b].iter().filter(|r| r["status"] == 1).count(),
        1,
        "{a} {b}"
    );
    let tokens_after: i32 = sqlx::query_scalar(
        "SELECT u.tokens FROM users u JOIN characters c ON c.user_id=u.id WHERE c.id=?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(tokens_after, tokens_before - 1050);
    let gold_before: i32 = sqlx::query_scalar("SELECT gold FROM characters WHERE id=?")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap();
    // Slot three needs five material_01. Set this synthetic account's balance
    // explicitly so unrelated market/reward checks cannot change this scenario.
    sqlx::query(
        "UPDATE character_items SET quantity=4 WHERE character_id=? AND item_id='material_01'",
    )
    .bind(id)
    .execute(pool)
    .await
    .unwrap();
    assert_eq!(
        amf_call(
            app,
            "PetService.executeService",
            json!(["learnSkill", [id, key, pet, 3, "mc1"]])
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
        gold_before
    );
    sqlx::query("UPDATE character_items SET quantity=quantity+5 WHERE character_id=? AND item_id='material_01'").bind(id).execute(pool).await.unwrap();
    let learned = amf_call(
        app,
        "PetService.executeService",
        json!(["learnSkill", [id, key, pet, 3, "mc1"]]),
    )
    .await;
    assert_eq!(learned["status"], 1, "{learned}");
    assert_eq!(
        sqlx::query_scalar::<_, i32>("SELECT gold FROM characters WHERE id=?")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap(),
        gold_before - 20000
    );
    let fresh = crate::configured_router(crate::GameState::with_database(Some(pool.clone())));
    let pets = amf_call(
        &fresh,
        "PetService.executeService",
        json!(["getPets", [id, key]]),
    )
    .await;
    assert_eq!(pets["pets"][0]["pet_skills"], "1,1,1,0,0,0");
    assert_eq!(
        amf_call(
            &fresh,
            "PetService.executeService",
            json!(["setLevel", [id, key, pet]])
        )
        .await["status"],
        2
    );
}
