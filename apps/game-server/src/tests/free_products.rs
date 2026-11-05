//! Former real-money products must deliver locally without deducting currency.
use super::amf_call;
use axum::Router;
use persistence::{MySqlPool, Row, sqlx};
use serde_json::json;
pub async fn run(app: &Router, pool: &MySqlPool, id: u64, key: &str) {
    let row=sqlx::query("SELECT c.user_id,c.gold,u.tokens,u.account_type FROM characters c JOIN users u ON u.id=c.user_id WHERE c.id=?").bind(id).fetch_one(pool).await.unwrap();
    let uid = row.get::<u64, _>("user_id");
    let before = row.get::<i32, _>("tokens");
    let request = format!("free-{}", authentication::random_hex().unwrap());
    let claim = amf_call(
        app,
        "Billing.claimProduct",
        json!([id, key, "id.ninjasage.tokens.500", request]),
    )
    .await;
    assert_eq!(claim["status"], 1, "{claim}");
    assert_eq!(claim["tokens"], before + 500);
    assert_eq!(claim["rewards"], json!(["tokens_500"]));
    let replay = amf_call(
        app,
        "Billing.claimProduct",
        json!([id, key, "id.ninjasage.tokens.500", request]),
    )
    .await;
    assert_eq!(replay["status"], 1);
    assert_eq!(replay["tokens"], before + 500);
    assert_eq!(replay["rewards"], json!([]));
    assert_eq!(
        amf_call(
            app,
            "Billing.claimProduct",
            json!([id, key, "id.ninjasage.tokens.50000", request])
        )
        .await["status"],
        2
    );
    assert_ne!(
        amf_call(
            app,
            "Billing.claimProduct",
            json!([id, "wrong", "id.ninjasage.emblem", "denied-claim"])
        )
        .await["status"],
        1
    );
    assert_eq!(
        amf_call(
            app,
            "Billing.claimProduct",
            json!([id, key, "id.ninjasage.tokens.2147483647", "invalid-claim"])
        )
        .await["status"],
        2
    );
    sqlx::query("UPDATE users SET account_type=0 WHERE id=?")
        .bind(uid)
        .execute(pool)
        .await
        .unwrap();
    let emblem = amf_call(
        app,
        "Billing.claimProduct",
        json!([id, key, "id.ninjasage.emblem", "free-emblem"]),
    )
    .await;
    assert_eq!(emblem["status"], 1, "{emblem}");
    assert_eq!(emblem["account_type"], 1);
    assert_eq!(emblem["rewards"], json!(["essential_10"]));
    assert_eq!(emblem["tokens"], before + 500);
    let ancestor = amf_call(
        app,
        "Billing.claimProduct",
        json!([id, key, "id.ninjasage.ancestor", "free-ancestor"]),
    )
    .await;
    assert_eq!(ancestor["status"], 1, "{ancestor}");
    assert!(
        ancestor["rewards"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "wpn_1341")
    );
    assert_eq!(ancestor["tokens"], before + 500);
    let gold: i32 = sqlx::query_scalar("SELECT gold FROM characters WHERE id=?")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(gold, row.get::<i32, _>("gold"));
    // Administrators must retain their stored role when claiming a free emblem.
    sqlx::query("UPDATE users SET account_type=99 WHERE id=?")
        .bind(uid)
        .execute(pool)
        .await
        .unwrap();
    let admin = amf_call(
        app,
        "Billing.claimProduct",
        json!([id, key, "id.ninjasage.trial.emblem", "free-admin-emblem"]),
    )
    .await;
    assert_eq!(admin["status"], 1, "{admin}");
    let role: i8 = sqlx::query_scalar("SELECT account_type FROM users WHERE id=?")
        .bind(uid)
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(role, 99);
    sqlx::query("UPDATE users SET account_type=?,tokens=? WHERE id=?")
        .bind(row.get::<i8, _>("account_type"))
        .bind(before)
        .bind(uid)
        .execute(pool)
        .await
        .unwrap();
}
