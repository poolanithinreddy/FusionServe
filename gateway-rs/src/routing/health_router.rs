//! Background health poller.
//!
//! Periodically probes each unique backend endpoint's health URL and updates the
//! shared `BackendHealth`. Routing consults that state so traffic is not sent to
//! a backend that is known-down; combined with the circuit breaker, this gives
//! both proactive (polled) and reactive (failure-driven) health signals.

use crate::config::Backend;
use crate::observability::Metrics;
use crate::routing::registry::Registry;
use std::sync::Arc;
use std::time::Duration;

/// Health endpoints per backend type.
/// - Triton (KServe v2): `GET /v2/health/ready`
/// - Dynamo frontend:    `GET /health`
fn health_url(backend: Backend, endpoint: &str) -> String {
    let base = endpoint.trim_end_matches('/');
    match backend {
        Backend::Triton => format!("{base}/v2/health/ready"),
        Backend::Dynamo => format!("{base}/health"),
    }
}

/// Spawn the health-polling loop. Returns immediately; the task runs until the
/// process exits.
pub fn spawn(
    registry: Arc<Registry>,
    metrics: Arc<Metrics>,
    client: reqwest::Client,
    poll_interval: Duration,
    unhealthy_after: usize,
) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(poll_interval);
        loop {
            ticker.tick().await;
            for backend in registry.backends() {
                let url = health_url(backend.backend, &backend.endpoint);
                let ok = match client
                    .get(&url)
                    .timeout(Duration::from_millis(1000))
                    .send()
                    .await
                {
                    Ok(resp) => resp.status().is_success(),
                    Err(_) => false,
                };
                backend.record_poll(ok, unhealthy_after);

                metrics
                    .backend_health
                    .with_label_values(&[backend.backend.as_str(), &backend.metric_id])
                    .set(if backend.is_healthy() { 1 } else { 0 });

                if !ok {
                    tracing::warn!(
                        backend = backend.backend.as_str(),
                        endpoint = %backend.endpoint,
                        healthy = backend.is_healthy(),
                        "backend health poll failed"
                    );
                }
            }
        }
    });
}
