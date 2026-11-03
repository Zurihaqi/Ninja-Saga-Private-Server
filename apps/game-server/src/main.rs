#[tokio::main]
async fn main() -> shared::ServerResult {
    let database = match std::env::var("DATABASE_URL") {
        Ok(url) => Some(
            persistence::connect(&url)
                .await
                .map_err(|_| "Unable to connect to configured database")?,
        ),
        Err(_) => None,
    };
    shared::run(
        "game-server",
        8080,
        game_server::configured_router(game_server::GameState::with_database(database)),
    )
    .await
}
