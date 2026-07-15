//! Backend clients.
//!
//! Both clients speak HTTP: Triton via the KServe v2 REST inference protocol,
//! Dynamo via its OpenAI-compatible frontend. gRPC transports (Triton gRPC,
//! documented in the registry as `protocol: grpc`) are a planned addition; see
//! docs/limitations.md. Clients classify failures into [`UpstreamError`] so the
//! pipeline can decide what is retriable and what should trip the breaker.

pub mod dynamo;
pub mod triton;

/// A failure talking to a backend, classified by what it means for retry and
/// circuit-breaker accounting.
#[derive(Debug)]
pub enum UpstreamError {
    /// Could not establish/complete the connection. Safe to retry for
    /// idempotent requests; counts as a breaker failure.
    Connect(String),
    /// The upstream did not respond within the deadline. Breaker failure.
    Timeout,
    /// Upstream returned a 5xx. Breaker failure (backend is unhealthy).
    ServerStatus { code: u16, body: String },
    /// Upstream returned a 4xx. The client's request is at fault — surfaced to
    /// the caller but does NOT trip the breaker.
    ClientStatus { code: u16, body: String },
    /// Response could not be parsed. Breaker failure (backend misbehaving).
    Malformed(String),
}

impl UpstreamError {
    /// Whether this failure indicates backend ill-health (should trip breaker).
    pub fn is_backend_fault(&self) -> bool {
        !matches!(self, UpstreamError::ClientStatus { .. })
    }

    /// Whether the request may be safely retried (connection-level only).
    pub fn is_retriable_connect(&self) -> bool {
        matches!(self, UpstreamError::Connect(_))
    }

    pub fn code_label(&self) -> &'static str {
        match self {
            UpstreamError::Connect(_) => "connect",
            UpstreamError::Timeout => "timeout",
            UpstreamError::ServerStatus { .. } => "server_status",
            UpstreamError::ClientStatus { .. } => "client_status",
            UpstreamError::Malformed(_) => "malformed",
        }
    }
}

impl std::fmt::Display for UpstreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpstreamError::Connect(m) => write!(f, "connect error: {m}"),
            UpstreamError::Timeout => write!(f, "upstream timed out"),
            UpstreamError::ServerStatus { code, .. } => write!(f, "upstream returned {code}"),
            UpstreamError::ClientStatus { code, .. } => {
                write!(f, "upstream rejected request ({code})")
            }
            UpstreamError::Malformed(m) => write!(f, "malformed upstream response: {m}"),
        }
    }
}

/// Convert a reqwest error at send-time into our classification.
pub(crate) fn classify_send_error(e: reqwest::Error) -> UpstreamError {
    if e.is_timeout() {
        UpstreamError::Timeout
    } else {
        // Connect/request/body transport failures are all connection-level as
        // far as retry accounting is concerned.
        UpstreamError::Connect(e.to_string())
    }
}

/// Convert an HTTP status into a classified error (caller has already checked
/// that the status is non-success).
pub(crate) fn classify_status(code: u16, body: String) -> UpstreamError {
    if (500..600).contains(&code) {
        UpstreamError::ServerStatus { code, body }
    } else {
        UpstreamError::ClientStatus { code, body }
    }
}
