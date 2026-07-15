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
        timeout: Duration,
    ) -> Result<Value, UpstreamError> {
        let url = format!(
            "{}/v2/models/{}/infer",
            endpoint.trim_end_matches('/'),
            model
        );

        let resp = self
            .http
            .post(&url)
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
