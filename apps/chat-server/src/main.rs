#[tokio::main]
async fn main() -> shared::ServerResult {
    shared::run("chat-server", 3002, axum::Router::new()).await
}
