//! Structured logging / tracing initialization.
//!
//! Logs are structured JSON by default (one object per line) so they can be
//! ingested by a log pipeline. Set `FUSIONSERVE_LOG_FORMAT=pretty` for
//! human-readable local development output. Verbosity follows `RUST_LOG`.

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn init() {
    let filter = EnvFilter::try_from_env("RUST_LOG").unwrap_or_else(|_| EnvFilter::new("info"));

    let pretty = std::env::var("FUSIONSERVE_LOG_FORMAT")
        .map(|v| v == "pretty")
        .unwrap_or(false);

    if pretty {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().json().flatten_event(true))
            .init();
    }
}
