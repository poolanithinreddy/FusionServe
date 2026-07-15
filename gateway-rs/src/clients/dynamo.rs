//! Dynamo client (OpenAI-compatible frontend).
//!
//! LLM chat traffic is sent to Dynamo's frontend at
//! `POST {endpoint}/v1/chat/completions`. Dynamo owns worker discovery and
//! KV-aware routing across vLLM workers; the gateway forwards the request and,
//! for streaming, proxies the Server-Sent-Events body straight through so the
//! client sees tokens as they are produced.
//!
//! Retry policy note: once a streaming response has begun we must NOT retry —
//! the client may already have received partial tokens. The pipeline enforces
//! this by only ever retrying the initial (pre-first-byte) connect failure and
//! only when the model's retry config allows it.

use super::{classify_send_error, classify_status, UpstreamError};
use serde_json::Value;
use std::time::Duration;

#[derive(Clone)]
pub struct DynamoClient {
    http: reqwest::Client,
}

impl DynamoClient {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }

    fn chat_url(endpoint: &str) -> String {
        format!("{}/v1/chat/completions", endpoint.trim_end_matches('/'))
    }

    /// Non-streaming chat completion. Returns the parsed JSON response.
    pub async fn chat_once(
        &self,
        endpoint: &str,
        body: &Value,
        timeout: Duration,
    ) -> Result<Value, UpstreamError> {
        let resp = self
            .http
            .post(Self::chat_url(endpoint))
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

    /// Begin a streaming chat completion. Returns the raw response so the caller
    /// can proxy the SSE byte stream (and measure time-to-first-token). Only the
    /// connection/headers phase is covered by `timeout`; the body may stream for
    /// longer, bounded by the overall request deadline in the handler.
    pub async fn chat_stream(
        &self,
        endpoint: &str,
        body: &Value,
        timeout: Duration,
    ) -> Result<reqwest::Response, UpstreamError> {
        let resp = self
            .http
            .post(Self::chat_url(endpoint))
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
        Ok(resp)
    }
}
