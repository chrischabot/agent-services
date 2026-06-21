//! Arcwell Memory server binary.
//!
//! Configure via `ARCWELL_MEMORY_CONFIG_FILE` (path to JSON) or
//! `ARCWELL_MEMORY_CONFIG` (inline JSON). Legacy `MEM0_CONFIG_FILE` and
//! `MEM0_CONFIG` are accepted as compatibility aliases.
//! Binds `0.0.0.0:$PORT` (default 8080).

use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let memory = Arc::new(arcwell_memory_server::build_memory_from_env()?);
    let app = arcwell_memory_server::app(memory);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("arcwell-memory-server listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
