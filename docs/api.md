# API Reference

Base URL (dev): `http://localhost:8080`. All bodies are JSON. Every response
carries an `x-request-id` header; provide your own with the `x-request-id`
request header or the gateway mints one. Shorten a request's deadline with
`x-deadline-ms` (it can only shorten, never extend, the server default).

## Error model

Every error returns a stable body so clients branch on `code`, not prose:

```json
{ "error": { "code": "circuit_open", "message": "circuit breaker open", "request_id": "…" } }
```

| code | HTTP | meaning |
|------|------|---------|
| `bad_request` | 400 | malformed body / wrong surface for model |
| `unknown_model` | 404 | model not in registry |
| `payload_too_large` | 413 | body exceeds model's `max_request_bytes` |
| `queue_full` | 429 | per-model waiting room full — back off |
| `queue_timeout` | 429 | waited past `queue_timeout_ms` |
| `overloaded` | 503 | global in-flight cap reached |
| `circuit_open` | 503 | breaker open for this backend |
| `backend_unhealthy` | 503 | backend down or admin-disabled |
| `deadline_exceeded` | 504 | upstream exceeded the deadline |
| `upstream_error` | 503 | upstream 5xx / unreachable |
| `upstream_malformed` | 502 | upstream returned unparseable data |
| `internal` | 502 | unexpected gateway error |

## Health

- `GET /healthz` → 200 if the process is alive.
- `GET /readyz` → 200 if ≥1 backend is routable, else 503 with per-backend detail.

## Model discovery

- `GET /v1/models` → `{ "object": "list", "data": [ … ] }`
- `GET /v1/models/{name}` → one model's capabilities + live health.

## Non-LLM inference (→ Triton)

- `POST /v1/infer/{model}` — body forwarded to Triton `/v2/models/{model}/infer`.
- `POST /v1/embeddings` — OpenAI-style; `{ "model": "...", "input": "..." }`.

```bash
curl localhost:8080/v1/infer/resnet50 -H 'content-type: application/json' \
  -d '{"inputs":[{"name":"input","datatype":"FP32","shape":[1,3,224,224],"data":[...]}]}'
```

## LLM chat (→ Dynamo), OpenAI-compatible

- `POST /v1/chat/completions`

```bash
# non-streaming
curl localhost:8080/v1/chat/completions -H 'content-type: application/json' \
  -d '{"model":"qwen_small","messages":[{"role":"user","content":"hi"}]}'

# streaming (SSE): data: {...}\n\n … data: [DONE]
curl -N localhost:8080/v1/chat/completions -H 'content-type: application/json' \
  -d '{"model":"qwen_small","stream":true,"messages":[{"role":"user","content":"hi"}]}'
```

Streaming responses are never retried once bytes have flowed. For `qwen_small`
the retry policy is `max_retries: 0`.

## Admin (development only — authenticate before exposing)

- `GET  /admin/backends` — health + enable state.
- `GET  /admin/routes` — model → backend table.
- `POST /admin/backends/{endpoint}/disable` — kill-switch (endpoint URL-encoded).
- `POST /admin/backends/{endpoint}/enable`.

## Metrics

- `GET /metrics` — Prometheus text exposition. See [benchmarking.md](benchmarking.md)
  and the metric list in [../README.md](../README.md).
