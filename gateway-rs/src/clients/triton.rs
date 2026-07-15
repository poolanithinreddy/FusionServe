//! Triton client (KServe v2 REST inference protocol).
//!
//! The gateway forwards the caller's inference body to
//! `POST {endpoint}/v2/models/{model}/infer` and returns Triton's response
//! verbatim. Triton owns batching, scheduling and model execution; the gateway
//! only adds admission control, deadlines, and observability around the call.

use super::{classify_send_error, classify_status, UpstreamError};
use serde_json::Value;
use std::time::Duration;

#[derive(Clone)]
pub struct TritonClient {
    http: reqwest::Client,
}

impl TritonClient {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }

    /// Forward an inference request. `body` is passed through unchanged so the
    /// caller controls the exact KServe tensor payload.
    pub async fn infer(
        &self,
        endpoint: &str,
        model: &str,
        body: &Value,
        request_id: &str,
        timeout: Duration,
    ) -> Result<Value, UpstreamError> {
        let url = infer_url(endpoint, model)?;

        let resp = self
            .http
            .post(url)
            .header("x-request-id", request_id)
            .json(body)
            .timeout(timeout)
            .send()
            .await
            .map_err(classify_send_error)?;

        let status = resp.status().as_u16();
        if !(200..300).contains(&status) {
            let text = resp.text().await.unwrap_or_default();
            return Err(classify_status(status, text));
        }

        resp.json::<Value>()
            .await
            .map_err(|e| UpstreamError::Malformed(e.to_string()))
    }
}

fn infer_url(endpoint: &str, model: &str) -> Result<reqwest::Url, UpstreamError> {
    let mut url = reqwest::Url::parse(endpoint)
        .map_err(|e| UpstreamError::Connect(format!("invalid Triton endpoint: {e}")))?;
    url.path_segments_mut()
        .map_err(|_| UpstreamError::Connect("Triton endpoint cannot be a base URL".into()))?
        .pop_if_empty()
        .extend(["v2", "models", model, "infer"]);
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs_encoded_kserve_model_url() {
        let url = infer_url("http://triton:8000/base/", "vision model/v1").unwrap();
        assert_eq!(
            url.as_str(),
            "http://triton:8000/base/v2/models/vision%20model%2Fv1/infer"
        );
    }

    #[test]
    fn rejects_non_base_endpoint() {
        assert!(infer_url("not a URL", "resnet50").is_err());
    }
}
