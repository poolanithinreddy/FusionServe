# Architecture

FusionServe is a single Rust gateway in front of two inference engines. The
gateway owns cross-cutting reliability and observability; the engines own model
execution. This document describes the components and the request lifecycle.

## Components

| Component | Owns | Implemented by |
|-----------|------|----------------|
| **Gateway** (Rust) | classification, registry, admission control, backpressure, timeouts, cancellation, circuit breaking, health-aware routing, unified metrics/tracing | this repo (`gateway-rs/`) |
| **Triton** | non-LLM model execution, dynamic batching, concurrent model execution, model load/unload, GPU inference metrics | NVIDIA Triton (integrated) |
| **Dynamo** | LLM worker discovery, KV-aware routing, multi-worker coordination, vLLM integration, LLM-serving metrics | NVIDIA Dynamo (integrated) |

The gateway does **not** reimplement Triton batching or Dynamo routing. It
routes to them and measures them. See [design-decisions.md](design-decisions.md).

## Request lifecycle

```
client → gateway.receive
       → classify (registry lookup; workload must match API surface)
       → admit:
           1. global in-flight cap        (else 503 overloaded)
           2. backend health / kill-switch (else 503 backend_unhealthy)
           3. circuit breaker allow()      (else 503 circuit_open)
           4. bounded queue + concurrency  (else 429 queue_full/queue_timeout)
       → forward to Triton (/v2/models/{m}/infer) or Dynamo (/v1/chat/completions)
           with an effective timeout = min(model_timeout, deadline_remaining)
           at most one connect-level retry (per model retry policy)
       → record breaker success/failure + metrics
       → respond (x-request-id echoed)
```

Every admission step returns an RAII guard. If the client disconnects, the
handler future is dropped, which drops the guards (releasing capacity) and the
in-flight `reqwest` call (cancelling the upstream request). Cancellation is thus
propagation-by-construction, not a special code path.

## Module map (`gateway-rs/src/`)

```
main.rs            binary: load config, build state, start health poller, serve
lib.rs             router wiring + build_state (shared with tests)
config.rs          YAML config model + validation
error.rs           unified error model (code → HTTP status → JSON body)
state.rs           AppState (config, registry, metrics, clients, global limiter)
routing/
  registry.rs      model runtimes + per-endpoint BackendHealth
  classifier.rs    workload/surface validation
  policy.rs        the admission + accounting pipeline (the reliability core)
  health_router.rs background health polling
admission/
  backpressure.rs  concurrency limiter + bounded queue (one atomic path)
  circuit_breaker.rs  closed/open/half-open breaker
clients/
  triton.rs        KServe v2 REST client
  dynamo.rs        OpenAI-compatible client (unary + streaming)
observability/
  metrics.rs       Prometheus metrics (bounded cardinality)
  tracing.rs       structured JSON logging
  request_context.rs  request id, deadline, routing labels
api/
  health.rs models.rs infer.rs chat.rs admin.rs
```

## Data flow diagram

See the top-level [README](../README.md#architecture) for the box diagram. The
non-LLM path terminates at Triton on the GPU; the LLM path terminates at the
Dynamo frontend, which fans out to one or more vLLM workers.
