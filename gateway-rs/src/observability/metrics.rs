//! Prometheus metrics for the gateway.
//!
//! Label cardinality is deliberately bounded to `{model, backend, workload,
//! status}`. We never label with request ids or prompt content — that would
//! explode cardinality and leak user data.

use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounterVec, IntGaugeVec, Registry, TextEncoder,
};

/// Handles to every metric, plus the registry they belong to.
pub struct Metrics {
    pub registry: Registry,

    pub requests_total: IntCounterVec,
    pub request_latency_seconds: HistogramVec,
    pub queue_wait_seconds: HistogramVec,
    pub inflight_requests: IntGaugeVec,
    pub queue_depth: IntGaugeVec,
    pub backend_errors_total: IntCounterVec,
    pub retries_total: IntCounterVec,
    /// 0 = closed, 1 = half-open, 2 = open.
    pub circuit_breaker_state: IntGaugeVec,
    pub request_cancellations_total: IntCounterVec,
    /// 1 = healthy, 0 = unhealthy.
    pub backend_health: IntGaugeVec,
    /// LLM time-to-first-token, seconds.
    pub llm_ttft_seconds: HistogramVec,
}

impl Metrics {
    pub fn new() -> anyhow::Result<Self> {
        let registry = Registry::new();

        let requests_total = IntCounterVec::new(
            prometheus::Opts::new("fusionserve_requests_total", "Total requests by outcome"),
            &["model", "backend", "workload", "status"],
        )?;
        let request_latency_seconds = HistogramVec::new(
            HistogramOpts::new(
                "fusionserve_request_latency_seconds",
                "End-to-end request latency",
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0,
            ]),
            &["model", "backend", "workload"],
        )?;
        let queue_wait_seconds = HistogramVec::new(
            HistogramOpts::new(
                "fusionserve_queue_wait_seconds",
                "Time spent waiting for an admission permit",
            )
            .buckets(vec![
                0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 2.0,
            ]),
            &["model", "backend", "workload"],
        )?;
        let inflight_requests = IntGaugeVec::new(
            prometheus::Opts::new("fusionserve_inflight_requests", "In-flight requests"),
            &["model", "backend"],
        )?;
        let queue_depth = IntGaugeVec::new(
            prometheus::Opts::new("fusionserve_queue_depth", "Current admission queue depth"),
            &["model", "backend"],
        )?;
        let backend_errors_total = IntCounterVec::new(
            prometheus::Opts::new("fusionserve_backend_errors_total", "Backend errors by code"),
            &["model", "backend", "code"],
        )?;
        let retries_total = IntCounterVec::new(
            prometheus::Opts::new("fusionserve_retries_total", "Retried backend calls"),
            &["model", "backend"],
        )?;
        let circuit_breaker_state = IntGaugeVec::new(
            prometheus::Opts::new(
                "fusionserve_circuit_breaker_state",
                "Circuit breaker state (0=closed,1=half_open,2=open)",
            ),
            &["model", "backend"],
        )?;
        let request_cancellations_total = IntCounterVec::new(
            prometheus::Opts::new(
                "fusionserve_request_cancellations_total",
                "Requests cancelled by client disconnect",
            ),
            &["model", "backend", "workload"],
        )?;
        let backend_health = IntGaugeVec::new(
            prometheus::Opts::new(
                "fusionserve_backend_health",
                "Backend health (1=healthy,0=unhealthy)",
            ),
            &["backend", "endpoint"],
        )?;
        let llm_ttft_seconds = HistogramVec::new(
            HistogramOpts::new("fusionserve_llm_ttft_seconds", "LLM time to first token")
                .buckets(vec![0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0]),
            &["model", "backend"],
        )?;

        registry.register(Box::new(requests_total.clone()))?;
        registry.register(Box::new(request_latency_seconds.clone()))?;
        registry.register(Box::new(queue_wait_seconds.clone()))?;
        registry.register(Box::new(inflight_requests.clone()))?;
        registry.register(Box::new(queue_depth.clone()))?;
        registry.register(Box::new(backend_errors_total.clone()))?;
        registry.register(Box::new(retries_total.clone()))?;
        registry.register(Box::new(circuit_breaker_state.clone()))?;
        registry.register(Box::new(request_cancellations_total.clone()))?;
        registry.register(Box::new(backend_health.clone()))?;
        registry.register(Box::new(llm_ttft_seconds.clone()))?;

        Ok(Self {
            registry,
            requests_total,
            request_latency_seconds,
            queue_wait_seconds,
            inflight_requests,
            queue_depth,
            backend_errors_total,
            retries_total,
            circuit_breaker_state,
            request_cancellations_total,
            backend_health,
            llm_ttft_seconds,
        })
    }

    /// Encode the registry in Prometheus text exposition format.
    pub fn encode(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buf = Vec::new();
        // Encoding into an in-memory buffer cannot fail in practice.
        let _ = encoder.encode(&metric_families, &mut buf);
        String::from_utf8(buf).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposition_contains_stable_metric_families() {
        let metrics = Metrics::new().unwrap();
        metrics
            .requests_total
            .with_label_values(&["resnet50", "triton", "image_classification", "ok"])
            .inc();
        let text = metrics.encode();
        assert!(text.contains("fusionserve_requests_total"));
        assert!(text.contains("model=\"resnet50\""));
        assert!(!text.contains("request_id"));
        assert!(!text.contains("prompt"));
    }
}
