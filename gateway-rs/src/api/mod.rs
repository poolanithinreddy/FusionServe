//! HTTP API surface and small shared helpers for building request context.

pub mod admin;
pub mod chat;
pub mod health;
pub mod infer;
pub mod models;

use crate::config::{Backend, Workload};
use crate::observability::request_context::{new_request_id, DEADLINE_HEADER, REQUEST_ID_HEADER};
use crate::observability::RequestContext;
use axum::http::HeaderMap;
use std::time::Duration;

/// Extract the client's request id or mint a fresh one.
pub fn request_id_from(headers: &HeaderMap) -> String {
    headers
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(new_request_id)
}

/// Resolve the request deadline: the client may shorten it via `x-deadline-ms`,
/// but never extend it past the server default.
fn resolve_deadline(headers: &HeaderMap, default: Duration) -> Duration {
    match headers
        .get(DEADLINE_HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
    {
        Some(ms) => Duration::from_millis(ms).min(default),
        None => default,
    }
}

/// Build the per-request context from headers and the resolved route.
pub fn build_context(
    headers: &HeaderMap,
    request_id: String,
    model: String,
    workload: Workload,
    backend: Backend,
    default_deadline: Duration,
) -> RequestContext {
    let deadline = resolve_deadline(headers, default_deadline);
    RequestContext::new(request_id, model, workload, backend, deadline)
}

/// Effective per-attempt upstream timeout: the smaller of the model's configured
/// timeout and the time left before the request deadline.
pub fn effective_timeout(ctx: &RequestContext, model_timeout: Duration) -> Duration {
    match ctx.remaining() {
        Some(rem) => rem.min(model_timeout),
        None => Duration::from_millis(0),
    }
}
