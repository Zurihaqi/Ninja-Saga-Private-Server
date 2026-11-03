#[tokio::main]
async fn main() -> shared::ServerResult {
    shared::run("protocol-spike", 8090, protocol_spike::router()).await
}
