//! Development/admin endpoints.
//!
//! WARNING: these mutate routing state and MUST NOT be exposed publicly without
//! authentication. They are intended for local development and controlled
//! operations. See docs/api.md.
//!
//! - `GET  /admin/backends`                  — backend health + enable state
//! - `GET  /admin/routes`                    — model → backend routing table
//! - `POST /admin/backends/{endpoint}/disable`
//! - `POST /admin/backends/{endpoint}/enable`
//!
//! `{endpoint}` is URL-encoded (e.g. `http%3A%2F%2Flocalhost%3A8001`).

use crate::error::{ErrorCode, GatewayError, GatewayResult};
use crate::state::SharedState;
use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};

pub async fn list_backends(State(state): State<SharedState>) -> Json<Value> {
    let backends: Vec<Value> = state
        .registry
        .backends()
        .map(|b| {
            json!({
                "backend": b.backend.as_str(),
                "endpoint": b.endpoint,
                "healthy": b.is_healthy(),
                "enabled": b.is_enabled(),
                "routable": b.is_routable(),
            })
        })
        .collect();
    Json(json!({ "backends": backends }))
}

pub async fn list_routes(State(state): State<SharedState>) -> Json<Value> {
    let routes: Vec<Value> = state
        .registry
        .models()
        .map(|m| {
            json!({
                "model": m.name,
                "workload": m.cfg.workload.as_str(),
                "backend": m.cfg.backend.as_str(),
                "endpoint": m.cfg.endpoint,
                "protocol": m.cfg.protocol.as_str(),
            })
        })
        .collect();
    Json(json!({ "routes": routes }))
}

pub async fn disable_backend(
    State(state): State<SharedState>,
    Path(endpoint): Path<String>,
) -> GatewayResult<Json<Value>> {
    set_backend_enabled(&state, &endpoint, false)
}

pub async fn enable_backend(
    State(state): State<SharedState>,
    Path(endpoint): Path<String>,
) -> GatewayResult<Json<Value>> {
    set_backend_enabled(&state, &endpoint, true)
}

fn set_backend_enabled(
    state: &SharedState,
    endpoint: &str,
    enabled: bool,
) -> GatewayResult<Json<Value>> {
    let backend = state.registry.backend(endpoint).ok_or_else(|| {
        GatewayError::new(
            ErrorCode::UnknownModel,
            format!("no backend with endpoint '{endpoint}'"),
        )
    })?;
    backend.set_enabled(enabled);
    tracing::info!(endpoint, enabled, "admin toggled backend");
    Ok(Json(json!({
        "endpoint": endpoint,
        "enabled": enabled,
        "routable": backend.is_routable(),
    })))
}
