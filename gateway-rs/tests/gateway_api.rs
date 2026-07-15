//! Integration tests for gateway behavior that does not require a live backend.
//!
//! These exercise the real router (built exactly as the binary builds it) via
//! `axum-test`, covering the error model, model discovery, admission pre-checks,
//! admin toggles, and metrics. End-to-end forwarding to Triton/Dynamo is covered
//! by the Python end-to-end test against the mock servers.

use axum_test::TestServer;
use fusionserve_gateway::{build_state, config::Config, router};
use serde_json::json;

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
