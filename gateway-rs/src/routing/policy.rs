//! The admission + accounting policy that every request flows through.
//!
//! This is where the reliability guarantees are enforced, in a fixed order:
//!
//! 1. Reject if the global in-flight cap is reached (`Overloaded`).
//! 2. Reject if the target backend is disabled or unhealthy.
//! 3. Reject fast if the circuit breaker is open.
//! 4. Admit through the per-model bounded queue (or reject `QueueFull`/
//!    `QueueTimeout`).
//! 5. Run the upstream call (with at most one connect-level retry, per config).
//! 6. Record breaker success/failure and all metrics.
//!
//! Steps 1–4 return RAII guards; dropping them — including on client
//! disconnect, when the handler future is cancelled — releases capacity.

use crate::admission::AdmitReject;
use crate::clients::UpstreamError;
use crate::config::RetryConfig;
use crate::error::{ErrorCode, GatewayError};
use crate::observability::RequestContext;
use crate::routing::Route;
use crate::state::SharedState;
use std::future::Future;
use tokio::sync::OwnedSemaphorePermit;

/// RAII bundle proving a request holds both the global and per-model slots.
pub struct Guards {
    _global: OwnedSemaphorePermit,
    _admission: crate::admission::AdmissionPermit,
}

/// Run steps 1–4. On success the returned guards must be held for the duration
/// of the upstream call.
pub async fn admit(
    state: &SharedState,
    route: &Route,
    ctx: &RequestContext,
) -> Result<Guards, GatewayError> {
    // 1. Global cap.
    let global = state
        .global_inflight
        .clone()
        .try_acquire_owned()
        .map_err(|_| {
            GatewayError::new(ErrorCode::Overloaded, "gateway at global capacity")
                .with_request_id(&ctx.request_id)
        })?;

    // 2. Backend health / admin kill-switch.
    if !route.model.health.is_routable() {
        let reason = if !route.model.health.is_enabled() {
            "backend administratively disabled"
        } else {
            "backend unhealthy"
        };
        return Err(
            GatewayError::new(ErrorCode::BackendUnhealthy, reason).with_request_id(&ctx.request_id)
        );
    }

    // 3. Circuit breaker.
    if !route.model.breaker.allow() {
        publish_breaker_state(state, route);
        return Err(
            GatewayError::new(ErrorCode::CircuitOpen, "circuit breaker open")
                .with_request_id(&ctx.request_id),
        );
    }

    // 4. Bounded queue + concurrency limit, timing the wait.
    let wait_start = std::time::Instant::now();
    let admission = route.model.admission.admit().await.map_err(|r| match r {
        AdmitReject::QueueFull => GatewayError::new(ErrorCode::QueueFull, "admission queue full")
            .with_request_id(&ctx.request_id),
        AdmitReject::QueueTimeout => {
            GatewayError::new(ErrorCode::QueueTimeout, "timed out waiting for capacity")
                .with_request_id(&ctx.request_id)
        }
        AdmitReject::Closed => GatewayError::new(ErrorCode::Overloaded, "gateway shutting down")
            .with_request_id(&ctx.request_id),
    })?;

    let labels = &[
        route.model.name.as_str(),
        route.backend.as_str(),
        route.workload.as_str(),
    ];
    state
        .metrics
        .queue_wait_seconds
        .with_label_values(labels)
        .observe(wait_start.elapsed().as_secs_f64());
    publish_gauges(state, route);

    Ok(Guards {
        _global: global,
        _admission: admission,
    })
}

/// Run an upstream future with an optional single connect-level retry.
/// `make_call` is invoked afresh on retry.
pub async fn run_with_retry<F, Fut, T>(
    state: &SharedState,
    route: &Route,
    retry: &RetryConfig,
    mut make_call: F,
) -> Result<T, UpstreamError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, UpstreamError>>,
{
    let mut attempt = 0usize;
    loop {
        let result = make_call().await;
        match result {
            Ok(v) => return Ok(v),
            Err(e) => {
                let can_retry = attempt < retry.max_retries
                    && (!retry.only_on_connect_error || e.is_retriable_connect());
                if can_retry {
                    attempt += 1;
                    state
                        .metrics
                        .retries_total
                        .with_label_values(&[route.model.name.as_str(), route.backend.as_str()])
                        .inc();
                    tracing::warn!(
                        model = %route.model.name,
                        attempt,
                        error = %e,
                        "retrying upstream call after connect-level failure"
                    );
                    continue;
                }
                return Err(e);
            }
        }
    }
}

/// Record the outcome of a completed request: breaker bookkeeping, counters,
/// latency, and (for LLM) is handled by the caller via `observe_ttft`.
pub fn record_result(
    state: &SharedState,
    route: &Route,
    ctx: &RequestContext,
    outcome: Result<(), &UpstreamError>,
) {
    let labels = &[
        route.model.name.as_str(),
        route.backend.as_str(),
        route.workload.as_str(),
    ];
    match outcome {
        Ok(()) => {
            route.model.breaker.on_success();
            state
                .metrics
                .requests_total
                .with_label_values(&[
                    route.model.name.as_str(),
                    route.backend.as_str(),
                    route.workload.as_str(),
                    "ok",
                ])
                .inc();
        }
        Err(e) => {
            if e.is_backend_fault() {
                route.model.breaker.on_failure();
            }
            state
                .metrics
                .backend_errors_total
                .with_label_values(&[
                    route.model.name.as_str(),
                    route.backend.as_str(),
                    e.code_label(),
                ])
                .inc();
            state
                .metrics
                .requests_total
                .with_label_values(&[
                    route.model.name.as_str(),
                    route.backend.as_str(),
                    route.workload.as_str(),
                    "error",
                ])
                .inc();
        }
    }
    state
        .metrics
        .request_latency_seconds
        .with_label_values(labels)
        .observe(ctx.elapsed().as_secs_f64());
    publish_breaker_state(state, route);
    publish_gauges(state, route);
}

/// Map an upstream failure to a client-facing gateway error.
pub fn map_upstream_error(e: &UpstreamError, request_id: &str) -> GatewayError {
    let ge = match e {
        UpstreamError::Connect(m) => GatewayError::new(ErrorCode::UpstreamError, m.clone()),
        UpstreamError::Timeout => {
            GatewayError::new(ErrorCode::DeadlineExceeded, "upstream deadline exceeded")
        }
        UpstreamError::ServerStatus { code, .. } => GatewayError::new(
            ErrorCode::UpstreamError,
            format!("upstream returned {code}"),
        ),
        UpstreamError::ClientStatus { code, body } => GatewayError::new(
            ErrorCode::BadRequest,
            format!("upstream rejected request ({code}): {body}"),
        ),
        UpstreamError::Malformed(m) => GatewayError::new(ErrorCode::UpstreamMalformed, m.clone()),
    };
    ge.with_request_id(request_id)
}

fn publish_gauges(state: &SharedState, route: &Route) {
    state
        .metrics
        .inflight_requests
        .with_label_values(&[route.model.name.as_str(), route.backend.as_str()])
        .set(route.model.admission.inflight());
    state
        .metrics
        .queue_depth
        .with_label_values(&[route.model.name.as_str(), route.backend.as_str()])
        .set(route.model.admission.queue_depth());
}

fn publish_breaker_state(state: &SharedState, route: &Route) {
    state
        .metrics
        .circuit_breaker_state
        .with_label_values(&[route.model.name.as_str(), route.backend.as_str()])
        .set(route.model.breaker.state().code());
}
