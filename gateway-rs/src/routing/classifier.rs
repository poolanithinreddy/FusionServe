//! Workload classification.
//!
//! FusionServe routes by *workload*, and the workload is a property of the
//! model as declared in the registry — not something we guess from payload
//! bytes. The classifier's job is therefore to (a) resolve a model to its
//! declared workload/backend and (b) enforce that the workload matches the API
//! surface it arrived on (e.g. a chat model may not be called via `/v1/infer`).

use crate::config::{Backend, Workload};
use crate::error::{ErrorCode, GatewayError};
use crate::routing::registry::{ModelRuntime, Registry};
use std::sync::Arc;

/// Which gateway API surface a request arrived on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiSurface {
    /// `POST /v1/infer/{model}`.
    Infer,
    /// `POST /v1/embeddings`.
    Embed,
    /// `POST /v1/chat/completions`.
    Chat,
}

/// The resolved routing decision for a request.
pub struct Route {
    pub model: Arc<ModelRuntime>,
    pub workload: Workload,
    pub backend: Backend,
}

/// Resolve a model name against the registry and verify it is valid for the
/// surface the request came in on.
pub fn classify(
    registry: &Registry,
    model_name: &str,
    surface: ApiSurface,
) -> Result<Route, GatewayError> {
    let model = registry
        .model(model_name)
        .ok_or_else(|| GatewayError::unknown_model(model_name))?;

    let workload = model.cfg.workload;
    let ok = match surface {
        ApiSurface::Infer => matches!(
            workload,
            Workload::ImageClassification | Workload::Embedding
        ),
        ApiSurface::Embed => matches!(workload, Workload::Embedding),
        ApiSurface::Chat => matches!(workload, Workload::ChatCompletion),
    };

    if !ok {
        return Err(GatewayError::new(
            ErrorCode::BadRequest,
            format!(
                "model '{model_name}' is a {} workload and cannot be called on this endpoint",
                workload.as_str()
            ),
        ));
    }

    Ok(Route {
        backend: model.cfg.backend,
        workload,
        model,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn test_registry() -> Registry {
        let yaml = r#"
server: { bind_addr: "0.0.0.0:8080", global_max_inflight: 10, default_deadline_ms: 1000 }
admission:
  queue_capacity: 4
  queue_timeout_ms: 100
  circuit_breaker: { failure_threshold: 3, open_cooldown_ms: 100, half_open_success_threshold: 1 }
health: { poll_interval_ms: 1000, unhealthy_after: 3 }
models:
  resnet50:
    workload: image_classification
    backend: triton
    protocol: http
    endpoint: "http://triton:8001"
    timeout_ms: 1000
    max_concurrency: 4
  qwen_small:
    workload: chat_completion
    backend: dynamo
    protocol: http
    endpoint: "http://dynamo:8000"
    timeout_ms: 5000
    max_concurrency: 4
    streaming: true
"#;
        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        Registry::build(&cfg)
    }

    #[test]
    fn routes_infer_to_triton() {
        let reg = test_registry();
        let route = classify(&reg, "resnet50", ApiSurface::Infer).unwrap();
        assert_eq!(route.backend, Backend::Triton);
        assert_eq!(route.workload, Workload::ImageClassification);
    }

    #[test]
    fn routes_chat_to_dynamo() {
        let reg = test_registry();
        let route = classify(&reg, "qwen_small", ApiSurface::Chat).unwrap();
        assert_eq!(route.backend, Backend::Dynamo);
    }

    #[test]
    fn rejects_chat_model_on_infer_surface() {
        let reg = test_registry();
        let err = classify(&reg, "qwen_small", ApiSurface::Infer)
            .err()
            .unwrap();
        assert_eq!(err.code, ErrorCode::BadRequest);
    }

    #[test]
    fn rejects_unknown_model() {
        let reg = test_registry();
        let err = classify(&reg, "nope", ApiSurface::Infer).err().unwrap();
        assert_eq!(err.code, ErrorCode::UnknownModel);
    }
}
