//! Non-LLM inference endpoints (routed to Triton).
//!
//! - `POST /v1/infer/{model}` — model name in the path, KServe-style body.
//! - `POST /v1/embeddings`    — OpenAI-style body with `model` inside it.
//!
//! Both funnel into one pipeline: classify → size-check → admit → forward →
//! record. The request id is echoed on every response (success or error).

use crate::api::{build_context, effective_timeout, request_id_from};
use crate::error::{ErrorCode, GatewayError};
use crate::observability::request_context::REQUEST_ID_HEADER;
use crate::routing::{classify, policy, ApiSurface};
use crate::state::SharedState;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::Value;

pub async fn infer(
    State(state): State<SharedState>,
    Path(model): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    run_infer(state, model, headers, body, ApiSurface::Infer).await
}

/// OpenAI-style embeddings: the model is named in the body.
pub async fn embeddings(
    State(state): State<SharedState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    // Peek only the model name; the full body is forwarded as-is.
    let model = match serde_json::from_slice::<Value>(&body)
        .ok()
        .and_then(|v| v.get("model").and_then(|m| m.as_str()).map(String::from))
    {
        Some(m) => m,
        None => {
            let rid = request_id_from(&headers);
            return GatewayError::bad_request("missing 'model' field")
                .with_request_id(rid)
                .into_response();
        }
    };
    run_infer(state, model, headers, body, ApiSurface::Embed).await
}

async fn run_infer(
    state: SharedState,
    model: String,
    headers: HeaderMap,
    body: Bytes,
    surface: ApiSurface,
) -> Response {
    let request_id = request_id_from(&headers);

    let route = match classify(&state.registry, &model, surface) {
        Ok(r) => r,
        Err(e) => return e.with_request_id(request_id).into_response(),
    };

    // Real byte-size enforcement against the model's configured limit.
    if body.len() > route.model.cfg.max_request_bytes {
        return GatewayError::new(
            ErrorCode::PayloadTooLarge,
            format!(
                "request body {} bytes exceeds limit {}",
                body.len(),
                route.model.cfg.max_request_bytes
            ),
        )
        .with_request_id(request_id)
        .into_response();
    }

    let payload: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return GatewayError::bad_request(format!("invalid JSON body: {e}"))
                .with_request_id(request_id)
                .into_response()
        }
    };

    let ctx = build_context(
        &headers,
        request_id.clone(),
        model.clone(),
        route.workload,
        route.backend,
        state.config.default_deadline(),
    );

    if ctx.is_expired() {
        return GatewayError::new(ErrorCode::DeadlineExceeded, "deadline already elapsed")
            .with_request_id(request_id)
            .into_response();
    }

    let _guards = match policy::admit(&state, &route, &ctx).await {
        Ok(g) => g,
        Err(e) => return e.into_response(),
    };

    let timeout = effective_timeout(&ctx, route.model.cfg.timeout());
    let retry = route.model.cfg.retry.clone();

    let result = policy::run_with_retry(&state, &route, &retry, || {
        state.triton.infer(
            &route.model.cfg.endpoint,
            &model,
            &payload,
            &ctx.request_id,
            timeout,
        )
    })
    .await;

    match &result {
        Ok(_) => policy::record_result(&state, &route, &ctx, Ok(())),
        Err(e) => policy::record_result(&state, &route, &ctx, Err(e)),
    }

    match result {
        Ok(value) => with_request_id(request_id, Json(value).into_response()),
        Err(e) => with_request_id(
            request_id.clone(),
            policy::map_upstream_error(&e, &request_id).into_response(),
        ),
    }
}

/// Attach the `x-request-id` header to a response for client-side correlation.
fn with_request_id(request_id: String, mut resp: Response) -> Response {
    if let Ok(val) = request_id.parse() {
        resp.headers_mut().insert(REQUEST_ID_HEADER, val);
    }
    resp
}
