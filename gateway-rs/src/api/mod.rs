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
        .filter(|s| {
            !s.is_empty()
                && s.len() <= 128
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn accepts_safe_correlation_id() {
        let mut headers = HeaderMap::new();
        headers.insert(
            REQUEST_ID_HEADER,
            HeaderValue::from_static("job_42.retry-1"),
        );
        assert_eq!(request_id_from(&headers), "job_42.retry-1");
    }

    #[test]
    fn replaces_oversized_or_unsafe_correlation_id() {
        let mut headers = HeaderMap::new();
        headers.insert(REQUEST_ID_HEADER, HeaderValue::from_static("unsafe/value"));
        assert_ne!(request_id_from(&headers), "unsafe/value");

        headers.insert(
            REQUEST_ID_HEADER,
            HeaderValue::from_str(&"a".repeat(129)).unwrap(),
        );
        assert_ne!(request_id_from(&headers), "a".repeat(129));
    }

    #[test]
    fn client_deadline_can_only_shorten_server_deadline() {
        let mut headers = HeaderMap::new();
        headers.insert(DEADLINE_HEADER, HeaderValue::from_static("250"));
        assert_eq!(
            resolve_deadline(&headers, Duration::from_secs(5)),
            Duration::from_millis(250)
        );

        headers.insert(DEADLINE_HEADER, HeaderValue::from_static("9000"));
        assert_eq!(
            resolve_deadline(&headers, Duration::from_secs(5)),
            Duration::from_secs(5)
        );
    }
}
