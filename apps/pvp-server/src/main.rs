#[tokio::main]
async fn main() -> shared::ServerResult {
    shared::run("pvp-server", 3000, axum::Router::new()).await
}
