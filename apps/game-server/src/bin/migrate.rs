//! Explicit additive migrations for an already provisioned runtime database.
#[tokio::main]
async fn main() -> shared::ServerResult {
    let url =
        std::env::var("DATABASE_URL").map_err(|_| "Set DATABASE_URL for the target database")?;
    let pool = persistence::connect(&url)
        .await
        .map_err(|_| "Unable to connect to target database")?;
    persistence::sqlx::raw_sql(include_str!(
        "../../../../database/migrations/001_pet_skills.sql"
    ))
    .execute(&pool)
    .await?;
    println!("Applied additive migration 001_pet_skills. Existing player rows were preserved.");
    Ok(())
}
