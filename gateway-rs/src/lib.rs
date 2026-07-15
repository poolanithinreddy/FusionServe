//! FusionServe gateway library.
//!
//! Exposed as a library so integration tests can build the same router the
//! binary serves. `main.rs` is a thin wrapper that loads config, constructs
//! [`state::AppState`], starts the health poller, and serves [`router`].

pub mod admission;
pub mod api;
pub mod clients;
pub mod config;
pub mod error;
pub mod observability;
pub mod routing;
pub mod state;

use axum::routing::{get, post};
use axum::Router;
use state::SharedState;
use tower_http::trace::TraceLayer;

/// Build the full application router.
pub fn router(state: SharedState) -> Router {
    Router::new()
        // Health
        .route("/healthz", get(api::health::healthz))
        .route("/readyz", get(api::health::readyz))
        // Metrics
        .route("/metrics", get(metrics_handler))
        // Model discovery
        .route("/v1/models", get(api::models::list_models))
        .route("/v1/models/:name", get(api::models::get_model))
        // Non-LLM inference
        .route("/v1/infer/:model", post(api::infer::infer))
        .route("/v1/infer", post(api::infer::infer_by_body))
        .route("/v1/embeddings", post(api::infer::embeddings))
        // LLM inference
        .route("/v1/chat/completions", post(api::chat::chat_completions))
        // Admin (dev only — see api::admin)
        .route("/admin/backends", get(api::admin::list_backends))
        .route("/admin/routes", get(api::admin::list_routes))
        .route(
            "/admin/backends/:endpoint/disable",
            post(api::admin::disable_backend),
        )
        .route(
            "/admin/backends/:endpoint/enable",
            post(api::admin::enable_backend),
        )
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &axum::http::Request<_>| {
                    tracing::info_span!(
                        "gateway.request",
                        method = %request.method(),
                        path = %request.uri().path(),
                    )
                })
                .on_response(
                    |response: &axum::http::Response<_>,
                     latency: std::time::Duration,
                     _span: &tracing::Span| {
                        tracing::info!(
                            status = response.status().as_u16(),
                            latency_ms = latency.as_secs_f64() * 1000.0,
                            "request completed"
                        );
                    },
                ),
        )
        .with_state(state)
}

async fn metrics_handler(
    axum::extract::State(state): axum::extract::State<SharedState>,
) -> impl axum::response::IntoResponse {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4",
        )],
        state.metrics.encode(),
    )
}

/// Construct application state from a loaded config, wiring the HTTP client used
/// for all backends. Shared by the binary and by integration tests.
pub fn build_state(config: config::Config) -> anyhow::Result<SharedState> {
    use std::sync::Arc;
    use std::time::Duration;

    let http = reqwest::Client::builder()
        .pool_max_idle_per_host(64)
        .connect_timeout(Duration::from_millis(1000))
        .build()?;

    let registry = Arc::new(routing::Registry::build(&config));
    let metrics = Arc::new(observability::Metrics::new()?);

    // Pre-create metric series so /metrics exposes every model/backend at 0
    // before any traffic, and so dashboards have stable series from startup.
    for (name, m) in &config.models {
        metrics
            .inflight_requests
            .with_label_values(&[name, m.backend.as_str()])
            .set(0);
        metrics
            .queue_depth
            .with_label_values(&[name, m.backend.as_str()])
            .set(0);
        metrics
            .circuit_breaker_state
            .with_label_values(&[name, m.backend.as_str()])
            .set(0);
        // Counter series start at 0 for both outcomes.
        for status in ["ok", "error"] {
            metrics
                .requests_total
                .with_label_values(&[name, m.backend.as_str(), m.workload.as_str(), status])
                .reset();
        }
    }
    for endpoint in config
        .models
        .values()
        .map(|m| (m.backend, m.endpoint.clone()))
        .collect::<std::collections::HashSet<_>>()
    {
        metrics
            .backend_health
            .with_label_values(&[endpoint.0.as_str(), &endpoint.1])
            .set(1);
    }

    let global_inflight = Arc::new(tokio::sync::Semaphore::new(
        config.server.global_max_inflight,
    ));

    Ok(Arc::new(state::AppState {
        config: Arc::new(config),
        registry,
        metrics,
        triton: clients::triton::TritonClient::new(http.clone()),
        dynamo: clients::dynamo::DynamoClient::new(http),
        global_inflight,
    }))
}
