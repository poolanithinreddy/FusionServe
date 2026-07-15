//! Shared application state handed to every handler.

use crate::clients::dynamo::DynamoClient;
use crate::clients::triton::TritonClient;
use crate::config::Config;
use crate::observability::Metrics;
use crate::routing::Registry;
use std::sync::Arc;
use tokio::sync::Semaphore;

pub struct AppState {
    pub config: Arc<Config>,
    pub registry: Arc<Registry>,
    pub metrics: Arc<Metrics>,
    pub triton: TritonClient,
    pub dynamo: DynamoClient,
    /// Global in-flight cap, a coarse backstop on top of per-model limits.
    pub global_inflight: Arc<Semaphore>,
}

pub type SharedState = Arc<AppState>;
