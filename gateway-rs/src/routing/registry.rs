//! The model-capability registry and per-model runtime state.
//!
//! Built once from config. Each model gets its own admission control and
//! circuit breaker; backend *health* is tracked per endpoint and shared by all
//! models pointing at that endpoint (e.g. resnet50 and text_embedding share one
//! Triton). Admin enable/disable is also per endpoint.

use crate::admission::{Admission, CircuitBreaker};
use crate::config::{Backend, Config, ModelConfig};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

/// Health and operational state for one backend endpoint.
pub struct BackendHealth {
    pub backend: Backend,
    pub endpoint: String,
    healthy: AtomicBool,
    /// Consecutive failed health polls (used by the health router).
    consecutive_poll_failures: AtomicUsize,
    /// Admin kill-switch. When false, all routing to this backend is rejected.
    enabled: AtomicBool,
}

impl BackendHealth {
    fn new(backend: Backend, endpoint: String) -> Self {
        Self {
            backend,
            endpoint,
            // Optimistically healthy until the first poll says otherwise.
            healthy: AtomicBool::new(true),
            consecutive_poll_failures: AtomicUsize::new(0),
            enabled: AtomicBool::new(true),
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::Relaxed)
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Routable = enabled by an operator AND observed healthy.
    pub fn is_routable(&self) -> bool {
        self.is_enabled() && self.is_healthy()
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Record a health-poll result. `unhealthy_after` consecutive failures flip
    /// the backend to unhealthy; a single success restores it.
    pub fn record_poll(&self, ok: bool, unhealthy_after: usize) {
        if ok {
            self.consecutive_poll_failures.store(0, Ordering::Relaxed);
            self.healthy.store(true, Ordering::Relaxed);
        } else {
            let n = self
                .consecutive_poll_failures
                .fetch_add(1, Ordering::Relaxed)
                + 1;
            if n >= unhealthy_after {
                self.healthy.store(false, Ordering::Relaxed);
            }
        }
    }
}

/// Everything needed to route and admit one model's traffic.
pub struct ModelRuntime {
    pub name: String,
    pub cfg: ModelConfig,
    pub admission: Admission,
    pub breaker: CircuitBreaker,
    pub health: Arc<BackendHealth>,
}

pub struct Registry {
    models: HashMap<String, Arc<ModelRuntime>>,
    /// Unique backend endpoints, keyed by endpoint URL.
    backends: HashMap<String, Arc<BackendHealth>>,
}

impl Registry {
    pub fn build(config: &Config) -> Self {
        let mut backends: HashMap<String, Arc<BackendHealth>> = HashMap::new();
        let mut models: HashMap<String, Arc<ModelRuntime>> = HashMap::new();

        for (name, m) in &config.models {
            let health = backends
                .entry(m.endpoint.clone())
                .or_insert_with(|| Arc::new(BackendHealth::new(m.backend, m.endpoint.clone())))
                .clone();

            let admission = Admission::new(
                m.max_concurrency,
                config.admission.queue_capacity,
                config.queue_timeout(),
            );
            let breaker = CircuitBreaker::new(&config.admission.circuit_breaker);

            models.insert(
                name.clone(),
                Arc::new(ModelRuntime {
                    name: name.clone(),
                    cfg: m.clone(),
                    admission,
                    breaker,
                    health,
                }),
            );
        }

        Self { models, backends }
    }

    pub fn model(&self, name: &str) -> Option<Arc<ModelRuntime>> {
        self.models.get(name).cloned()
    }

    pub fn models(&self) -> impl Iterator<Item = &Arc<ModelRuntime>> {
        self.models.values()
    }

    pub fn model_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.models.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn backends(&self) -> impl Iterator<Item = &Arc<BackendHealth>> {
        self.backends.values()
    }

    /// Look up a backend's health state by endpoint URL.
    pub fn backend(&self, endpoint: &str) -> Option<Arc<BackendHealth>> {
        self.backends.get(endpoint).cloned()
    }
}
