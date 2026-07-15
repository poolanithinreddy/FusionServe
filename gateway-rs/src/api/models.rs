//! Model discovery endpoints.
//!
//! `GET /v1/models` lists the registry; `GET /v1/models/{name}` returns one
//! model's capabilities and live health. These read from config-derived state,
//! so what a client sees is exactly what the gateway will route.

use crate::error::{GatewayError, GatewayResult};
use crate::state::SharedState;
use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};

fn model_view(rt: &crate::routing::ModelRuntime) -> Value {
    json!({
        "name": rt.name,
        "workload": rt.cfg.workload.as_str(),
        "backend": rt.cfg.backend.as_str(),
        "protocol": rt.cfg.protocol.as_str(),
        "streaming": rt.cfg.streaming,
        "max_concurrency": rt.cfg.max_concurrency,
        "timeout_ms": rt.cfg.timeout_ms,
        "healthy": rt.health.is_routable(),
        "circuit_state": rt.breaker.state().code(),
    })
}

pub async fn list_models(State(state): State<SharedState>) -> Json<Value> {
    let mut models: Vec<Value> = state.registry.models().map(|m| model_view(m)).collect();
    models.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    Json(json!({ "object": "list", "data": models }))
}

pub async fn get_model(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> GatewayResult<Json<Value>> {
    let rt = state
        .registry
        .model(&name)
        .ok_or_else(|| GatewayError::unknown_model(&name))?;
    Ok(Json(model_view(&rt)))
}
