//! FusionServe gateway binary.

use clap::Parser;
use fusionserve_gateway::observability::tracing as fs_tracing;
use fusionserve_gateway::routing::health_router;
use fusionserve_gateway::{build_state, config::Config, router};
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(name = "fusionserve-gateway", version, about)]
struct Args {
    /// Path to the YAML config file. Falls back to $FUSIONSERVE_CONFIG then
    /// ./config.yaml.
    #[arg(long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    fs_tracing::init();
    let args = Args::parse();

    let config_path = args
        .config
        .or_else(|| std::env::var("FUSIONSERVE_CONFIG").ok())
        .unwrap_or_else(|| "config.yaml".to_string());

    let config = Config::load(&config_path)?;
    let bind_addr = config.server.bind_addr.clone();
    let poll_interval = Duration::from_millis(config.health.poll_interval_ms);
    let unhealthy_after = config.health.unhealthy_after;

    tracing::info!(config = %config_path, %bind_addr, models = config.models.len(), "starting FusionServe gateway");

    let state = build_state(config)?;

    // Background backend health polling.
    let health_client = reqwest::Client::builder()
        .connect_timeout(Duration::from_millis(1000))
        .build()?;
    health_router::spawn(
        state.registry.clone(),
        state.metrics.clone(),
        health_client,
        poll_interval,
        unhealthy_after,
    );

    let app = router(state);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!(%bind_addr, "listening");

    // Graceful shutdown on Ctrl-C.
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}
