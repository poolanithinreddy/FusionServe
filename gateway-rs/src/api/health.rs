//! Liveness and readiness endpoints.
//!
//! - `/healthz` (liveness): the process is up. Always 200 unless we're dead.
//! - `/readyz` (readiness): at least one backend is routable. Returns 503 when
//!   no backend can currently serve traffic, so a load balancer / k8s probe
//!   stops sending requests we would only reject.

use crate::state::SharedState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde_json::json;

pub async fn healthz() -> StatusCode {
    StatusCode::OK
}

pub async fn readyz(State(state): State<SharedState>) -> (StatusCode, Json<serde_json::Value>) {
    let mut any_routable = false;
    let backends: Vec<_> = state
        .registry
        .backends()
        .map(|b| {
            let routable = b.is_routable();
            any_routable |= routable;
            json!({
                "backend": b.backend.as_str(),
                "endpoint": b.endpoint,
                "healthy": b.is_healthy(),
                "enabled": b.is_enabled(),
            })
        })
        .collect();

    let status = if any_routable {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(json!({ "ready": any_routable, "backends": backends })),
    )
}
