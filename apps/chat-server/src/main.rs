use chat_server::{ChatConfig, ChatServer, router, spawn_policy_server};

#[tokio::main]
async fn main() -> shared::ServerResult {
    let pool = match std::env::var("DATABASE_URL") {
        Ok(url) if !url.trim().is_empty() => Some(persistence::connect(&url).await?),
        _ => None,
    };
    let server = ChatServer::new(pool, ChatConfig::default());
    server.ensure_schema().await?;
    let policy_addr =
        std::env::var("POLICY_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:843".to_owned());
    spawn_policy_server(&policy_addr);
    shared::run("chat-server", 3002, router(server)).await
}
