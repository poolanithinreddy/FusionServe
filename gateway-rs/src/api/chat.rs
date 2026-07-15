//! LLM chat completions (routed to Dynamo), OpenAI-compatible.
//!
//! `POST /v1/chat/completions`. If the body sets `"stream": true` and the model
//! supports streaming, the gateway proxies Dynamo's SSE body straight through,
//! measuring time-to-first-token. The admission permit is *moved into the
//! stream* so capacity is held for the whole generation and released when the
//! stream ends or the client disconnects.
//!
//! Retry safety: streaming responses are never retried once bytes have flowed.
//! The model's retry config for `qwen_small` is `max_retries: 0`, so even the
//! initial connect is not retried by default for LLM traffic.

use crate::api::{build_context, effective_timeout, request_id_from};
use crate::error::{ErrorCode, GatewayError};
use crate::observability::request_context::REQUEST_ID_HEADER;
use crate::observability::RequestContext;
use crate::routing::{classify, policy, ApiSurface, Route};
use crate::state::SharedState;
use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{header, HeaderMap};
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures::StreamExt;
use prometheus::IntCounter;
use serde_json::Value;
use std::time::Instant;

struct StreamLifecycle {
    _guards: policy::Guards,
    cancellation_counter: IntCounter,
    completed: bool,
}

impl Drop for StreamLifecycle {
    fn drop(&mut self) {
        if !self.completed {
            self.cancellation_counter.inc();
            tracing::info!("streaming client disconnected before completion");
        }
    }
}

pub async fn chat_completions(
    State(state): State<SharedState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let request_id = request_id_from(&headers);

    let payload: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return GatewayError::bad_request(format!("invalid JSON body: {e}"))
                .with_request_id(request_id)
                .into_response()
        }
    };

    let model = match payload.get("model").and_then(|m| m.as_str()) {
        Some(m) => m.to_string(),
        None => {
            return GatewayError::bad_request("missing 'model' field")
                .with_request_id(request_id)
                .into_response()
        }
    };

    let route = match classify(&state.registry, &model, ApiSurface::Chat) {
        Ok(r) => r,
        Err(e) => return e.with_request_id(request_id).into_response(),
    };

    if body.len() > route.model.cfg.max_request_bytes {
        return GatewayError::new(ErrorCode::PayloadTooLarge, "request body too large")
            .with_request_id(request_id)
            .into_response();
    }

    let want_stream = payload
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if want_stream && !route.model.cfg.streaming {
        return GatewayError::bad_request("model does not support streaming")
            .with_request_id(request_id)
            .into_response();
    }

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

    let guards = match policy::admit(&state, &route, &ctx).await {
        Ok(g) => g,
        Err(e) => return e.into_response(),
    };

    if want_stream {
        stream_chat(state, route, ctx, payload, request_id, guards).await
    } else {
        unary_chat(state, route, ctx, payload, request_id, guards).await
    }
}

async fn unary_chat(
    state: SharedState,
    route: Route,
    ctx: RequestContext,
    payload: Value,
    request_id: String,
    _guards: policy::Guards,
) -> Response {
    let timeout = effective_timeout(&ctx, route.model.cfg.timeout());
    let retry = route.model.cfg.retry.clone();

    let result = policy::run_with_retry(&state, &route, &retry, || {
        state.dynamo.chat_once(
            &route.model.cfg.endpoint,
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
        Err(e) => {
            let resp = policy::map_upstream_error(&e, &request_id).into_response();
            with_request_id(request_id, resp)
        }
    }
}

async fn stream_chat(
    state: SharedState,
    route: Route,
    ctx: RequestContext,
    payload: Value,
    request_id: String,
    guards: policy::Guards,
) -> Response {
    let timeout = effective_timeout(&ctx, route.model.cfg.timeout());

    // Streaming requests are never retried; open the stream once.
    let resp = match state
        .dynamo
        .chat_stream(
            &route.model.cfg.endpoint,
            &payload,
            &ctx.request_id,
            timeout,
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            policy::record_result(&state, &route, &ctx, Err(&e));
            let resp = policy::map_upstream_error(&e, &request_id).into_response();
            return with_request_id(request_id, resp);
        }
    };

    // The upstream stream is now open. We consider the request a breaker success
    // once headers are received; body-level errors are logged but not retried.
    policy::record_result(&state, &route, &ctx, Ok(()));

    let model_name = route.model.name.clone();
    let backend = route.backend;
    let metrics = state.metrics.clone();
    let start = Instant::now();
    let cancellation_counter = metrics.request_cancellations_total.with_label_values(&[
        model_name.as_str(),
        backend.as_str(),
        route.workload.as_str(),
    ]);
    let lifecycle = StreamLifecycle {
        _guards: guards,
        cancellation_counter,
        completed: false,
    };
    let upstream = Box::pin(resp.bytes_stream());
    let mapped = futures::stream::unfold(
        (upstream, lifecycle, false),
        move |(mut upstream, mut lifecycle, first_seen)| {
            let metrics = metrics.clone();
            let model_name = model_name.clone();
            async move {
                match upstream.next().await {
                    Some(Ok(bytes)) => {
                        if !first_seen {
                            metrics
                                .llm_ttft_seconds
                                .with_label_values(&[model_name.as_str(), backend.as_str()])
                                .observe(start.elapsed().as_secs_f64());
                        }
                        if bytes
                            .windows(b"data: [DONE]".len())
                            .any(|w| w == b"data: [DONE]")
                        {
                            lifecycle.completed = true;
                        }
                        Some((Ok(bytes), (upstream, lifecycle, true)))
                    }
                    Some(Err(error)) => {
                        lifecycle.completed = true;
                        Some((
                            Err(std::io::Error::other(error.to_string())),
                            (upstream, lifecycle, first_seen),
                        ))
                    }
                    None => {
                        lifecycle.completed = true;
                        drop(lifecycle);
                        None
                    }
                }
            }
        },
    );

    let mut response = Response::builder()
        .status(200)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from_stream(mapped))
        .unwrap_or_else(|_| GatewayError::internal("failed to build stream").into_response());
    if let Ok(val) = request_id.parse() {
        response.headers_mut().insert(REQUEST_ID_HEADER, val);
    }
    response
}

fn with_request_id(request_id: String, mut resp: Response) -> Response {
    if let Ok(val) = request_id.parse() {
        resp.headers_mut().insert(REQUEST_ID_HEADER, val);
    }
    resp
}
