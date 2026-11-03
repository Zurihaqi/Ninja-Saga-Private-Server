//! Process infrastructure only; gameplay readiness is deliberately separate from liveness.
use axum::{Router, http::StatusCode, routing::get};
use std::{error::Error, net::SocketAddr};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

pub type ServerResult = Result<(), Box<dyn Error + Send + Sync>>;

pub fn bind_address(value: &str) -> Result<SocketAddr, std::net::AddrParseError> {
    value.parse()
}

pub fn health_router() -> Router {
    Router::new()
        .route("/health/live", get(|| async { "ok" }))
        .route(
            "/health/ready",
            get(|| async {
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "protocol and database compatibility not verified",
                )
            }),
        )
}

pub async fn run(service: &'static str, default_port: u16, routes: Router) -> ServerResult {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .json()
        .try_init()?;
    let address =
        std::env::var("BIND_ADDR").unwrap_or_else(|_| format!("127.0.0.1:{default_port}"));
    let listener = TcpListener::bind(bind_address(&address)?).await?;
    tracing::info!(service, address = %listener.local_addr()?, "starting foundation service");
    axum::serve(listener, health_router().merge(routes))
        .with_graceful_shutdown(shutdown())
        .await?;
    Ok(())
}

async fn shutdown() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install Ctrl-C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { () = ctrl_c => {}, () = terminate => {} }
    tracing::info!("shutdown requested");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bind_config_requires_explicit_ip_and_valid_port() {
        assert!(bind_address("127.0.0.1:8080").is_ok());
        assert!(bind_address("[::1]:8080").is_ok());
        assert!(bind_address("localhost:8080").is_err());
        assert!(bind_address("0.0.0.0:99999").is_err());
    }
}
