//! Integration tests for gateway behavior that does not require a live backend.
//!
//! These exercise the real router (built exactly as the binary builds it) via
//! `axum-test`, covering the error model, model discovery, admission pre-checks,
//! admin toggles, and metrics. End-to-end forwarding to Triton/Dynamo is covered
//! by the Python end-to-end test against the mock servers.

use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use axum_test::TestServer;
use fusionserve_gateway::{build_state, config::Config, router};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

#[derive(Clone, Default)]
struct BackendEvidence {
    request_ids: Arc<Mutex<Vec<String>>>,
}

async fn triton_backend(
    State(evidence): State<BackendEvidence>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> Response {
    evidence.request_ids.lock().await.push(
        headers
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_owned(),
    );
    match payload.get("mode").and_then(|value| value.as_str()) {
        Some("malformed") => (StatusCode::OK, "not-json").into_response(),
        Some("server_error") => (StatusCode::SERVICE_UNAVAILABLE, "down").into_response(),
        Some("client_error") => (StatusCode::BAD_REQUEST, "bad tensor").into_response(),
        Some("slow") => {
            tokio::time::sleep(Duration::from_millis(150)).await;
            Json(json!({"outputs": [{"data": [1.0]}]})).into_response()
        }
        _ => Json(json!({"outputs": [{"data": [0.71, 0.12]}]})).into_response(),
    }
}

async fn dynamo_backend(
    State(evidence): State<BackendEvidence>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> Response {
    evidence.request_ids.lock().await.push(
        headers
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_owned(),
    );
    match payload.get("mode").and_then(|value| value.as_str()) {
        Some("malformed") => (StatusCode::OK, "not-json").into_response(),
        Some("server_error") => (StatusCode::BAD_GATEWAY, "worker unavailable").into_response(),
        _ if payload.get("stream").and_then(|value| value.as_bool()) == Some(true) => {
            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/event-stream")
                .body(Body::from(
                    "data: {\"choices\":[{\"delta\":{\"content\":\"hello\"}}]}\n\ndata: [DONE]\n\n",
                ))
                .unwrap()
        }
        _ => Json(json!({
            "choices": [{"message": {"role": "assistant", "content": "hello"}}],
            "echo_temperature": payload.get("temperature"),
            "echo_max_tokens": payload.get("max_tokens")
        }))
        .into_response(),
    }
}

async fn spawn_backends() -> (
    String,
    String,
    BackendEvidence,
    tokio::task::JoinHandle<()>,
    tokio::task::JoinHandle<()>,
) {
    let evidence = BackendEvidence::default();
    let triton_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let triton_endpoint = format!("http://{}", triton_listener.local_addr().unwrap());
    let triton_app = Router::new()
        .route("/v2/models/:model/infer", post(triton_backend))
        .with_state(evidence.clone());
    let triton_handle = tokio::spawn(async move {
        axum::serve(triton_listener, triton_app).await.unwrap();
    });

    let dynamo_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let dynamo_endpoint = format!("http://{}", dynamo_listener.local_addr().unwrap());
    let dynamo_app = Router::new()
        .route("/v1/chat/completions", post(dynamo_backend))
        .with_state(evidence.clone());
    let dynamo_handle = tokio::spawn(async move {
        axum::serve(dynamo_listener, dynamo_app).await.unwrap();
    });
    (
        triton_endpoint,
        dynamo_endpoint,
        evidence,
        triton_handle,
        dynamo_handle,
    )
}

fn live_config(triton_endpoint: &str, dynamo_endpoint: &str) -> Config {
    let yaml = format!(
        r#"
server: {{ bind_addr: "127.0.0.1:0", global_max_inflight: 100, default_deadline_ms: 2000 }}
admission:
  queue_capacity: 8
  queue_timeout_ms: 200
  circuit_breaker: {{ failure_threshold: 2, open_cooldown_ms: 20, half_open_success_threshold: 1 }}
health: {{ poll_interval_ms: 60000, unhealthy_after: 3 }}
models:
  resnet50:
    workload: image_classification
    backend: triton
    protocol: http
    endpoint: "{triton_endpoint}"
    timeout_ms: 50
    max_concurrency: 8
    max_request_bytes: 1024
  qwen_small:
    workload: chat_completion
    backend: dynamo
    protocol: http
    endpoint: "{dynamo_endpoint}"
    timeout_ms: 500
    max_concurrency: 4
    streaming: true
  qwen_unary:
    workload: chat_completion
    backend: dynamo
    protocol: http
    endpoint: "{dynamo_endpoint}"
    timeout_ms: 500
    max_concurrency: 4
    streaming: false
"#
    );
    serde_yaml::from_str(&yaml).unwrap()
}

async fn live_server() -> (
    TestServer,
    BackendEvidence,
    Vec<tokio::task::JoinHandle<()>>,
) {
    let (triton, dynamo, evidence, triton_handle, dynamo_handle) = spawn_backends().await;
    let state = build_state(live_config(&triton, &dynamo)).unwrap();
    (
        TestServer::new(router(state)).unwrap(),
        evidence,
        vec![triton_handle, dynamo_handle],
    )
}

fn test_config() -> Config {
    // Endpoints intentionally point at ports nothing is listening on; every test
    // here fails or succeeds before any backend call is made.
    let yaml = r#"
server: { bind_addr: "127.0.0.1:0", global_max_inflight: 100, default_deadline_ms: 2000 }
admission:
  queue_capacity: 8
  queue_timeout_ms: 200
  circuit_breaker: { failure_threshold: 3, open_cooldown_ms: 500, half_open_success_threshold: 1 }
health: { poll_interval_ms: 60000, unhealthy_after: 3 }
models:
  resnet50:
    workload: image_classification
    backend: triton
    protocol: http
    endpoint: "http://127.0.0.1:59001"
    timeout_ms: 500
    max_concurrency: 8
    max_request_bytes: 1024
  qwen_small:
    workload: chat_completion
    backend: dynamo
    protocol: http
    endpoint: "http://127.0.0.1:59000"
    timeout_ms: 2000
    max_concurrency: 4
    streaming: true
"#;
    serde_yaml::from_str(yaml).unwrap()
}

fn server() -> TestServer {
    let state = build_state(test_config()).unwrap();
    TestServer::new(router(state)).unwrap()
}

#[tokio::test]
async fn healthz_is_ok() {
    let server = server();
    let resp = server.get("/healthz").await;
    resp.assert_status_ok();
}

#[tokio::test]
async fn lists_models() {
    let server = server();
    let resp = server.get("/v1/models").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    let names: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"resnet50"));
    assert!(names.contains(&"qwen_small"));
}

#[tokio::test]
async fn unknown_model_returns_404_with_stable_code() {
    let server = server();
    let resp = server
        .post("/v1/infer/does_not_exist")
        .json(&json!({"inputs": []}))
        .await;
    resp.assert_status(axum::http::StatusCode::NOT_FOUND);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["error"]["code"], "unknown_model");
    assert!(body["error"]["request_id"].is_string());
}

#[tokio::test]
async fn chat_model_rejected_on_infer_surface() {
    let server = server();
    let resp = server
        .post("/v1/infer/qwen_small")
        .json(&json!({"inputs": []}))
        .await;
    resp.assert_status(axum::http::StatusCode::BAD_REQUEST);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["error"]["code"], "bad_request");
}

#[tokio::test]
async fn unified_infer_requires_model_in_body() {
    let server = server();
    let resp = server.post("/v1/infer").json(&json!({"inputs": []})).await;
    resp.assert_status(axum::http::StatusCode::BAD_REQUEST);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["error"]["code"], "bad_request");
    assert!(body["error"]["request_id"].is_string());
}

#[tokio::test]
async fn unified_infer_classifies_model_from_body() {
    let server = server();
    let resp = server
        .post("/v1/infer")
        .json(&json!({"model": "does_not_exist", "inputs": []}))
        .await;
    resp.assert_status(axum::http::StatusCode::NOT_FOUND);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["error"]["code"], "unknown_model");
}

#[tokio::test]
async fn payload_too_large_is_413() {
    let server = server();
    // resnet50 has a 1024-byte limit.
    let big = "x".repeat(4096);
    let resp = server
        .post("/v1/infer/resnet50")
        .json(&json!({"inputs": big}))
        .await;
    resp.assert_status(axum::http::StatusCode::PAYLOAD_TOO_LARGE);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["error"]["code"], "payload_too_large");
}

#[tokio::test]
async fn request_id_is_echoed_from_header() {
    let server = server();
    let resp = server
        .post("/v1/infer/does_not_exist")
        .add_header("x-request-id", "abc-123")
        .json(&json!({"inputs": []}))
        .await;
    let body: serde_json::Value = resp.json();
    assert_eq!(body["error"]["request_id"], "abc-123");
}

#[tokio::test]
async fn admin_disable_makes_backend_unroutable() {
    let server = server();
    // Disable the Triton backend by its endpoint.
    let ep = "http://127.0.0.1:59001";
    let resp = server
        .post(&format!("/admin/backends/{}/disable", urlencoding(ep)))
        .await;
    resp.assert_status_ok();

    // Now inference to a Triton model is rejected as backend_unhealthy (503)
    // before any upstream call.
    let resp = server
        .post("/v1/infer/resnet50")
        .json(&json!({"inputs": [1, 2, 3]}))
        .await;
    resp.assert_status(axum::http::StatusCode::SERVICE_UNAVAILABLE);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["error"]["code"], "backend_unhealthy");
}

#[tokio::test]
async fn readyz_reflects_all_backends_disabled() {
    let server = server();
    for ep in ["http://127.0.0.1:59001", "http://127.0.0.1:59000"] {
        server
            .post(&format!("/admin/backends/{}/disable", urlencoding(ep)))
            .await
            .assert_status_ok();
    }
    let resp = server.get("/readyz").await;
    resp.assert_status(axum::http::StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn metrics_endpoint_exposes_prometheus_text() {
    let server = server();
    let resp = server.get("/metrics").await;
    resp.assert_status_ok();
    let text = resp.text();
    assert!(text.contains("fusionserve_requests_total"));
}

/// Minimal percent-encoding for the endpoint path segment used in admin routes.
fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u32),
        })
        .collect()
}

#[tokio::test]
async fn forwards_triton_success_and_request_id() {
    let (server, evidence, _handles) = live_server().await;
    let response = server
        .post("/v1/infer/resnet50")
        .add_header("x-request-id", "triton-request-1")
        .json(&json!({"inputs": [{"data": [1.0]}]}))
        .await;
    response.assert_status_ok();
    assert_eq!(response.header("x-request-id"), "triton-request-1");
    assert_eq!(
        response.json::<serde_json::Value>()["outputs"][0]["data"][0],
        0.71
    );
    assert_eq!(
        evidence.request_ids.lock().await.as_slice(),
        ["triton-request-1"]
    );
}

#[tokio::test]
async fn classifies_triton_malformed_client_server_and_timeout_failures() {
    for (mode, status, code) in [
        ("malformed", StatusCode::BAD_GATEWAY, "upstream_malformed"),
        (
            "server_error",
            StatusCode::SERVICE_UNAVAILABLE,
            "upstream_error",
        ),
        ("client_error", StatusCode::BAD_REQUEST, "bad_request"),
        ("slow", StatusCode::GATEWAY_TIMEOUT, "deadline_exceeded"),
    ] {
        let (server, _evidence, _handles) = live_server().await;
        let response = server
            .post("/v1/infer/resnet50")
            .json(&json!({"mode": mode, "inputs": []}))
            .await;
        response.assert_status(status);
        assert_eq!(response.json::<serde_json::Value>()["error"]["code"], code);
    }
}

#[tokio::test]
async fn forwards_unary_dynamo_parameters_and_request_id() {
    let (server, evidence, _handles) = live_server().await;
    let response = server
        .post("/v1/chat/completions")
        .add_header("x-request-id", "chat-request-1")
        .json(&json!({
            "model": "qwen_small",
            "messages": [{"role": "user", "content": "hello"}],
            "temperature": 0.25,
            "max_tokens": 17
        }))
        .await;
    response.assert_status_ok();
    assert_eq!(response.header("x-request-id"), "chat-request-1");
    let body: serde_json::Value = response.json();
    assert_eq!(body["echo_temperature"], 0.25);
    assert_eq!(body["echo_max_tokens"], 17);
    assert_eq!(
        evidence.request_ids.lock().await.as_slice(),
        ["chat-request-1"]
    );
}

#[tokio::test]
async fn proxies_dynamo_sse_and_records_ttft() {
    let (server, _evidence, _handles) = live_server().await;
    let response = server
        .post("/v1/chat/completions")
        .add_header("x-request-id", "stream-request-1")
        .json(&json!({
            "model": "qwen_small",
            "stream": true,
            "messages": [{"role": "user", "content": "hello"}]
        }))
        .await;
    response.assert_status_ok();
    assert_eq!(response.header("content-type"), "text/event-stream");
    assert_eq!(response.header("x-request-id"), "stream-request-1");
    assert!(response.text().contains("data: [DONE]"));

    let metrics = server.get("/metrics").await.text();
    assert!(metrics.contains("fusionserve_llm_ttft_seconds_count"));
    assert!(metrics.contains("model=\"qwen_small\""));
}

#[tokio::test]
async fn rejects_invalid_chat_requests_before_backend() {
    let (server, evidence, _handles) = live_server().await;
    let invalid_json = server
        .post("/v1/chat/completions")
        .content_type("application/json")
        .bytes(b"{".to_vec().into())
        .await;
    invalid_json.assert_status_bad_request();

    server
        .post("/v1/chat/completions")
        .json(&json!({"messages": []}))
        .await
        .assert_status_bad_request();

    server
        .post("/v1/chat/completions")
        .json(&json!({"model": "qwen_unary", "stream": true, "messages": []}))
        .await
        .assert_status_bad_request();
    assert!(evidence.request_ids.lock().await.is_empty());
}

#[tokio::test]
async fn classifies_dynamo_malformed_and_server_failures() {
    let (server, _evidence, _handles) = live_server().await;
    for (mode, code) in [
        ("malformed", "upstream_malformed"),
        ("server_error", "upstream_error"),
    ] {
        let response = server
            .post("/v1/chat/completions")
            .json(&json!({"model": "qwen_small", "mode": mode, "messages": []}))
            .await;
        let expected_status = if mode == "malformed" {
            StatusCode::BAD_GATEWAY
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        };
        response.assert_status(expected_status);
        assert_eq!(response.json::<serde_json::Value>()["error"]["code"], code);
    }
}
