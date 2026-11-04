use pvp_server::{PvpConfig, PvpServer, router};

#[tokio::main]
async fn main() -> shared::ServerResult {
    let pool = match std::env::var("DATABASE_URL") {
        Ok(url) if !url.trim().is_empty() => Some(persistence::connect(&url).await?),
        _ => None,
    };
    let server = PvpServer::new(pool, PvpConfig::default());
    shared::run("pvp-server", 3000, router(server)).await
}
