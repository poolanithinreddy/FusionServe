//! Per-request context threaded through the gateway.
//!
//! It carries the request id (generated if the client did not supply one), the
//! absolute deadline, and the resolved routing labels. Every span and metric is
//! derived from this, and the request id is echoed on the response and in errors.

use crate::config::{Backend, Workload};
use std::time::{Duration, Instant};

pub const REQUEST_ID_HEADER: &str = "x-request-id";
pub const DEADLINE_HEADER: &str = "x-deadline-ms";

#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: String,
    pub model: String,
    pub workload: Workload,
    pub backend: Backend,
    /// Absolute instant by which the request must complete.
    pub deadline: Instant,
    /// When the request entered the gateway (for total-latency accounting).
    pub received_at: Instant,
}

impl RequestContext {
    pub fn new(
        request_id: String,
        model: String,
        workload: Workload,
        backend: Backend,
        deadline: Duration,
    ) -> Self {
        let now = Instant::now();
        Self {
            request_id,
            model,
            workload,
            backend,
            deadline: now + deadline,
            received_at: now,
        }
    }

    /// Remaining time before the deadline, or `None` if already past it.
    pub fn remaining(&self) -> Option<Duration> {
        self.deadline.checked_duration_since(Instant::now())
    }

    pub fn is_expired(&self) -> bool {
        self.remaining().is_none()
    }

    pub fn elapsed(&self) -> Duration {
        self.received_at.elapsed()
    }
}

/// Generate a fresh request id.
pub fn new_request_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
