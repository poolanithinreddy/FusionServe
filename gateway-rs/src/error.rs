//! Unified error model for the gateway.
//!
//! Every failure the gateway can produce maps to exactly one HTTP status and a
//! stable JSON body `{ "error": { "code", "message", "request_id" } }`, so
//! clients (Python and C++) can branch on `code` rather than parsing prose.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

/// Machine-stable error codes. These are part of the API contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// Request body/params failed validation.
    BadRequest,
    /// Model name is not in the registry.
    UnknownModel,
    /// Request larger than the model's configured limit.
    PayloadTooLarge,
    /// Per-model queue is full — the gateway is shedding load.
    QueueFull,
    /// Request waited in the queue longer than the queue timeout.
    QueueTimeout,
    /// Global in-flight cap reached.
    Overloaded,
    /// Circuit breaker is open for this model's backend.
    CircuitOpen,
    /// Selected backend is currently unhealthy.
    BackendUnhealthy,
    /// Upstream call exceeded the request deadline.
    DeadlineExceeded,
    /// Upstream returned an error or was unreachable.
    UpstreamError,
    /// Upstream produced a malformed/unparseable response.
    UpstreamMalformed,
    /// Anything unexpected inside the gateway.
    Internal,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::BadRequest => "bad_request",
            ErrorCode::UnknownModel => "unknown_model",
            ErrorCode::PayloadTooLarge => "payload_too_large",
            ErrorCode::QueueFull => "queue_full",
            ErrorCode::QueueTimeout => "queue_timeout",
            ErrorCode::Overloaded => "overloaded",
            ErrorCode::CircuitOpen => "circuit_open",
            ErrorCode::BackendUnhealthy => "backend_unhealthy",
            ErrorCode::DeadlineExceeded => "deadline_exceeded",
            ErrorCode::UpstreamError => "upstream_error",
            ErrorCode::UpstreamMalformed => "upstream_malformed",
            ErrorCode::Internal => "internal",
        }
    }

    pub fn status(self) -> StatusCode {
        match self {
            ErrorCode::BadRequest => StatusCode::BAD_REQUEST,
            ErrorCode::UnknownModel => StatusCode::NOT_FOUND,
            ErrorCode::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            // Load-shedding: 429 tells clients to back off and retry later.
            ErrorCode::QueueFull | ErrorCode::QueueTimeout => StatusCode::TOO_MANY_REQUESTS,
            // Capacity/health problems: 503 is retriable against another replica.
            ErrorCode::Overloaded
            | ErrorCode::CircuitOpen
            | ErrorCode::BackendUnhealthy
            | ErrorCode::UpstreamError => StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::DeadlineExceeded => StatusCode::GATEWAY_TIMEOUT,
            ErrorCode::UpstreamMalformed | ErrorCode::Internal => StatusCode::BAD_GATEWAY,
        }
    }
}

/// A gateway error carrying its code, a human message, and (once known) the
/// request id so the response is self-describing for correlation with traces.
#[derive(Debug, Clone)]
pub struct GatewayError {
    pub code: ErrorCode,
    pub message: String,
    pub request_id: Option<String>,
}

impl GatewayError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            request_id: None,
        }
    }

    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }

    // Convenience constructors used throughout the handlers.
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::BadRequest, msg)
    }
    pub fn unknown_model(name: &str) -> Self {
        Self::new(ErrorCode::UnknownModel, format!("unknown model '{name}'"))
    }
    pub fn upstream(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::UpstreamError, msg)
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::Internal, msg)
    }
}

impl std::fmt::Display for GatewayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for GatewayError {}

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        let status = self.code.status();
        let body = Json(json!({
            "error": {
                "code": self.code.as_str(),
                "message": self.message,
                "request_id": self.request_id,
            }
        }));
        (status, body).into_response()
    }
}

pub type GatewayResult<T> = Result<T, GatewayError>;
