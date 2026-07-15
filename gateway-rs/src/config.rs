//! Configuration model, loaded from YAML at startup.
//!
//! The model-capability registry is configuration-driven on purpose: adding or
//! retuning a model is a config change, not a code change (no `if model == ...`).

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub admission: AdmissionConfig,
    pub health: HealthConfig,
    pub models: HashMap<String, ModelConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub global_max_inflight: usize,
    pub default_deadline_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdmissionConfig {
    pub queue_capacity: usize,
    pub queue_timeout_ms: u64,
    pub circuit_breaker: CircuitBreakerConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: usize,
    pub open_cooldown_ms: u64,
    pub half_open_success_threshold: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HealthConfig {
    pub poll_interval_ms: u64,
    pub unhealthy_after: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Workload {
    ImageClassification,
    Embedding,
    ChatCompletion,
}

impl Workload {
    pub fn as_str(self) -> &'static str {
        match self {
            Workload::ImageClassification => "image_classification",
            Workload::Embedding => "embedding",
            Workload::ChatCompletion => "chat_completion",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    Triton,
    Dynamo,
}

impl Backend {
    pub fn as_str(self) -> &'static str {
        match self {
            Backend::Triton => "triton",
            Backend::Dynamo => "dynamo",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Http,
    Grpc,
}

impl Protocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Protocol::Http => "http",
            Protocol::Grpc => "grpc",
        }
    }
}

fn default_max_request_bytes() -> usize {
    1 << 20 // 1 MiB
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelConfig {
    pub workload: Workload,
    pub backend: Backend,
    pub protocol: Protocol,
    pub endpoint: String,
    pub timeout_ms: u64,
    pub max_concurrency: usize,
    #[serde(default)]
    pub streaming: bool,
    #[serde(default = "default_max_request_bytes")]
    pub max_request_bytes: usize,
    #[serde(default)]
    pub retry: RetryConfig,
}

impl ModelConfig {
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RetryConfig {
    #[serde(default)]
    pub max_retries: usize,
    #[serde(default = "default_true")]
    pub only_on_connect_error: bool,
}

fn default_true() -> bool {
    true
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 0,
            only_on_connect_error: true,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config file {path}: {source}")]
    Read {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to parse config: {0}")]
    Parse(#[from] serde_yaml::Error),
    #[error("invalid config: {0}")]
    Invalid(String),
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path_ref = path.as_ref();
        let raw = std::fs::read_to_string(path_ref).map_err(|source| ConfigError::Read {
            path: path_ref.display().to_string(),
            source,
        })?;
        let cfg: Config = serde_yaml::from_str(&raw)?;
        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.models.is_empty() {
            return Err(ConfigError::Invalid("no models configured".into()));
        }
        for (name, m) in &self.models {
            if m.max_concurrency == 0 {
                return Err(ConfigError::Invalid(format!(
                    "model '{name}' has max_concurrency = 0"
                )));
            }
            // Chat is the only streaming-capable workload we support today.
            if m.streaming && m.workload != Workload::ChatCompletion {
                return Err(ConfigError::Invalid(format!(
                    "model '{name}' enables streaming but is not a chat_completion workload"
                )));
            }
        }
        Ok(())
    }

    pub fn queue_timeout(&self) -> Duration {
        Duration::from_millis(self.admission.queue_timeout_ms)
    }

    pub fn default_deadline(&self) -> Duration {
        Duration::from_millis(self.server.default_deadline_ms)
    }
}
