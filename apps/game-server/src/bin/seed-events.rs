//! Explicit, non-destructive event catalog provisioning. Existing admin records win.
use persistence::sqlx;
use serde_json::Value;
#[tokio::main]
async fn main() -> shared::ServerResult {
    let url =
        std::env::var("DATABASE_URL").map_err(|_| "Set DATABASE_URL for the target database")?;
    let pool = persistence::connect(&url)
        .await
        .map_err(|_| "Unable to connect to target database")?;
    let catalog: Vec<Value> =
        serde_json::from_str(include_str!("../../../../database/catalogs/events.json"))?;
    let supported = [
        "ChristmasMenu",
        "FeastOfGratitudeMenu",
        "ConfrontingDeathMenu",
        "HalloweenMenu",
        "MonsterHunter",
        "DragonHunt",
        "JusticeBadge",
        "GiveawayCenter",
        "Leaderboard",
        "ExoticPackage",
    ];
    let mut tx = pool.begin().await?;
    let mut inserted = 0;
    for event in catalog {
        let title = event["title"].as_str().ok_or("Invalid event title")?;
        let panel = event["panel"].as_str();
        let exists: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM game_events WHERE panel <=> ? AND title=?")
                .bind(panel)
                .bind(title)
                .fetch_one(&mut *tx)
                .await?;
        if exists > 0 {
            continue;
        }
        let active = panel.is_none_or(|p| supported.contains(&p));
        inserted+=sqlx::query("INSERT INTO game_events(title,`type`,panel,`date`,icon,inside,image_url,description,`data`,sort_order,active,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,NOW(),NOW())")
            .bind(title).bind(event["type"].as_str().unwrap_or("feature")).bind(panel).bind(event["date"].as_str()).bind(event["icon"].as_str().map(str::to_owned).or_else(|| event["icon"].as_i64().map(|n|n.to_string()))).bind(event["inside"].as_bool().unwrap_or(false)).bind(event["image_url"].as_str().unwrap_or("")).bind(event["description"].as_str()).bind(event.get("data").cloned().unwrap_or(serde_json::json!({}))).bind(event["sort_order"].as_i64().unwrap_or(0)).bind(active).execute(&mut *tx).await?.rows_affected();
    }
    tx.commit().await?;
    println!(
        "Provisioned {inserted} missing event records. Existing records preserved; unsupported panels remain inactive."
    );
    Ok(())
}
