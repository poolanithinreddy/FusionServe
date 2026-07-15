//! Admission control: concurrency limiting, bounded queueing, backpressure, and
//! circuit breaking. The concurrency limiter and bounded queue are implemented
//! together in [`backpressure`] because they share one atomic accounting path;
//! the circuit breaker is independent.

pub mod backpressure;
pub mod circuit_breaker;

pub use backpressure::{Admission, AdmissionPermit, AdmitReject};
pub use circuit_breaker::{CircuitBreaker, State as CircuitState};
