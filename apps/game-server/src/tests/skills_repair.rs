use super::amf_call;
use axum::Router;
use persistence::{MySqlPool, Row, sqlx};
use serde_json::json;

/// Exercises the normal skill panel's exact preset prices and save/equip payloads.
/// Called only by the existing isolated MySQL regression.
pub async fn run(app: &Router, pool: &MySqlPool, owner: u64, key: &str) {
    let user=sqlx::query("SELECT u.id,u.account_type,u.tokens FROM users u JOIN characters c ON c.user_id=u.id WHERE c.id=?").bind(owner).fetch_one(pool).await.unwrap();
    let uid = user.get::<u64, _>("id");
    let account = user.get::<i8, _>("account_type");
    let original_tokens = user.get::<i32, _>("tokens");
    let name = format!("skill_repair_{}", authentication::random_hex().unwrap());
    let id =
        sqlx::query("INSERT INTO characters(user_id,name,equipment_skills) VALUES(?,?,'skill_13')")
            .bind(uid)
            .bind(name)
            .execute(pool)
            .await
            .unwrap()
            .last_insert_id();
    for skill in ["skill_13", "skill_01"] {
        sqlx::query("INSERT INTO character_skills(character_id,skill_id) VALUES(?,?)")
            .bind(id)
            .bind(skill)
            .execute(pool)
            .await
            .unwrap();
    }
    sqlx::query("UPDATE users SET account_type=0,tokens=10000 WHERE id=?")
        .bind(uid)
        .execute(pool)
        .await
        .unwrap();
    let first = amf_call(app, "CharacterService.createSkillSet", json!([id, key])).await;
    assert_eq!(first["status"], 1, "{first}");
    let preset = first["skillsets"][0]["id"].as_u64().unwrap();
    let balance = || {
        sqlx::query_scalar::<_, i32>("SELECT tokens FROM users WHERE id=?")
            .bind(uid)
            .fetch_one(pool)
    };
    assert_eq!(balance().await.unwrap(), 10000);
    assert_eq!(
        amf_call(app, "CharacterService.createSkillSet", json!([id, key])).await["status"],
        1
    );
    assert_eq!(balance().await.unwrap(), 9400);
    sqlx::query("UPDATE users SET account_type=1 WHERE id=?")
        .bind(uid)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        amf_call(app, "CharacterService.createSkillSet", json!([id, key])).await["status"],
        1
    );
    assert_eq!(balance().await.unwrap(), 9200);
    assert_eq!(
        amf_call(app, "CharacterService.createSkillSet", json!([id, key])).await["status"],
        1
    );
    assert_eq!(balance().await.unwrap(), 9000);
    assert_eq!(
        amf_call(app, "CharacterService.createSkillSet", json!([id, key])).await["status"],
        2
    );
    assert_eq!(balance().await.unwrap(), 9000);
    for loadout in ["", "skill_13,skill_13", "skill_not_owned"] {
        let rejected = amf_call(
            app,
            "CharacterService.equipSkillSet",
            json!([id, key, loadout]),
        )
        .await;
        assert_eq!(rejected["status"], 2, "{rejected}");
        assert!(rejected["result"].as_str().is_some());
    }
    let invalid = amf_call(
        app,
        "CharacterService.saveSkillSet",
        json!([id, key, preset, "skill_not_owned"]),
    )
    .await;
    assert_eq!(invalid["status"], 2);
    let saved = amf_call(
        app,
        "CharacterService.saveSkillSet",
        json!([id, key, preset, " skill_01,,skill_13,"]),
    )
    .await;
    assert_eq!(saved["status"], 1, "{saved}");
    assert_eq!(saved["skillsets"][0]["skills"], "skill_01,skill_13");
    assert_eq!(
        amf_call(
            app,
            "CharacterService.equipSkillSet",
            json!([id, key, "skill_13,skill_01"])
        )
        .await["status"],
        1
    );
    let fresh = crate::configured_router(crate::GameState::with_database(Some(pool.clone())));
    let loaded = amf_call(&fresh, "SystemLogin.getCharacterData", json!([id, key])).await;
    assert_eq!(loaded["character_sets"]["skills"], "skill_13,skill_01");
    sqlx::query("DELETE FROM characters WHERE id=?")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE users SET account_type=?,tokens=? WHERE id=?")
        .bind(account)
        .bind(original_tokens)
        .bind(uid)
        .execute(pool)
        .await
        .unwrap();
}
