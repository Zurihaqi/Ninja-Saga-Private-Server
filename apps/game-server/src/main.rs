#[tokio::main]
async fn main() -> shared::ServerResult {
    shared::run("game-server", 8080, game_server::router()).await
}
