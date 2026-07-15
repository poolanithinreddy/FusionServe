//! Routing: the model registry, workload classifier, and background health
//! poller that together decide where a request goes and whether the target is
//! currently accepting traffic.

pub mod classifier;
pub mod health_router;
pub mod policy;
pub mod registry;

pub use classifier::{classify, ApiSurface, Route};
pub use registry::{BackendHealth, ModelRuntime, Registry};
