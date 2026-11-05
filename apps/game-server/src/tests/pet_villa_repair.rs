//! Exact PetVilla payload and training-state regression on a synthetic account.
use super::amf_call;
use axum::Router;
use persistence::{MySqlPool, sqlx};
use serde_json::json;

pub async fn run(app: &Router, pool: &MySqlPool, id: u64, key: &str) {
    let level: i32 = sqlx::query_scalar("SELECT level FROM characters WHERE id=?")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap();
    let pet=sqlx::query("INSERT INTO character_pets(character_id,pet_id,name,level,xp,maturity_points,created_at,updated_at) VALUES(?,'pet_regression','Villa contract pet',1,0,0,NOW(),NOW())").bind(id).execute(pool).await.unwrap().last_insert_id();
    let villa = amf_call(
        app,
        "PetService.executeService",
        json!(["getVillaData", [id, key]]),
    )
    .await;
    assert_eq!(villa["status"], 1, "{villa}");
    assert_eq!(villa["slots"].as_array().unwrap().len(), 4);
    let entry = villa["pets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["pet_id"] == pet)
        .unwrap();
    // Type ID and movie filename differ in this fixture; use the catalog movie.
    assert_eq!(entry["pet_swf"], "pet_01");
    assert_eq!(entry["train_gold"], 2000);
    assert_eq!(entry["train_time_text"], "24m");
    let gold: i32 = sqlx::query_scalar("SELECT gold FROM characters WHERE id=?")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap();
    let started = amf_call(
        app,
        "PetService.executeService",
        json!(["trainPet", [id, key, pet]]),
    )
    .await;
    assert_eq!(started["status"], 1, "{started}");
    assert_eq!(started["gold"], gold - 2000);
    assert_eq!(
        amf_call(
            app,
            "PetService.executeService",
            json!(["trainPet", [id, key, pet]])
        )
        .await["status"],
        2
    );
    let refreshed = amf_call(
        app,
        "PetService.executeService",
        json!(["getVillaData", [id, key]]),
    )
    .await;
    let slot = refreshed["slots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["pet_id"] == pet)
        .unwrap();
    assert_eq!(slot["status"], 2);
    assert!((1..=1440).contains(&slot["completed_at"].as_i64().unwrap()));
    let skipped = amf_call(
        app,
        "PetService.executeService",
        json!(["skipTraining", [id, key, pet]]),
    )
    .await;
    assert_eq!(skipped["status"], 1, "{skipped}");
    assert!(skipped["tokens"].as_i64().is_some());
    assert_eq!(
        amf_call(
            app,
            "PetService.executeService",
            json!(["checkoutPet", [id, key, pet]])
        )
        .await["status"],
        1
    );
    assert_ne!(
        amf_call(
            app,
            "PetService.executeService",
            json!(["checkoutPet", [id, key, pet]])
        )
        .await["status"],
        1
    );
    let trained: i32 = sqlx::query_scalar("SELECT level FROM character_pets WHERE id=?")
        .bind(pet)
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(trained, 2);
    sqlx::query("UPDATE character_pets SET level=? WHERE id=?")
        .bind(level)
        .bind(pet)
        .execute(pool)
        .await
        .unwrap();
    let capped = amf_call(
        app,
        "PetService.executeService",
        json!(["trainPet", [id, key, pet]]),
    )
    .await;
    assert_eq!(capped["status"], 2, "{capped}");
    assert!(capped["result"].as_str().unwrap().contains("level cap"));
    sqlx::query("DELETE FROM character_pets WHERE id=?")
        .bind(pet)
        .execute(pool)
        .await
        .unwrap();
}
