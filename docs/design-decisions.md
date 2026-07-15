# Design Decisions

Short ADR-style notes on the choices that shape FusionServe.

## 1. The gateway integrates engines; it does not reimplement them

Triton already provides dynamic batching, concurrent model execution, and model
management; Dynamo already provides KV-aware routing and worker coordination.
Reimplementing those would be worse and dishonest. The gateway's original value
is the **cross-cutting control plane**: admission control, backpressure,
deadlines, cancellation, circuit breaking, health-aware routing, and unified
observability across two very different engines. Claims are worded accordingly
(see [limitations.md](limitations.md)).

## 2. Configuration-driven registry, not hardcoded routing

Adding or retuning a model is a YAML change, not a code change. `classifier.rs`
resolves a model to its declared workload/backend and validates it against the
API surface — no `if model == "..."`.

## 3. Admission ordering is fixed and cheap-checks-first

Global cap → backend health → circuit breaker → bounded queue. The cheapest,
most-likely-to-reject checks run first, so an overloaded gateway spends the least
work rejecting. Each step yields an RAII guard so capacity is released on any
exit path, including client disconnect.

## 4. Concurrency limit + bounded queue in one atomic path

`backpressure.rs` uses a `tokio::Semaphore` for the concurrency limit and a
single atomic for the bounded waiting room. A request reserves a queue slot,
waits for a permit up to `queue_timeout`, then transitions to in-flight. This
gives a hard cap on both in-flight *and* waiting requests per model — overload
becomes a fast 429/503, never unbounded latency.

## 5. Circuit breaker distinguishes backend faults from client faults

Upstream 5xx / connect / timeout trip the breaker; upstream 4xx does not (that's
the caller's bad request, not backend ill-health). This keeps the breaker a
signal of *backend* health. See `clients/mod.rs::UpstreamError::is_backend_fault`.

## 6. Retry policy is conservative and stream-aware

At most one retry, connect-level only, and only if the model's config allows it.
LLM streaming is never retried once bytes flow (`qwen_small` uses
`max_retries: 0`). This avoids duplicate side effects and double-billing tokens.

## 7. HTTP transport first, gRPC later

Both engines expose HTTP (Triton KServe v2 REST; Dynamo OpenAI-compatible). HTTP
keeps the build free of a `protoc`/gRPC toolchain and is correct and complete for
the workloads here. gRPC (Triton's `:8001`) is a planned addition; the registry
already models `protocol: grpc` so it's a client-implementation change, not a
schema change.

## 8. Bounded metric cardinality

Labels are `{model, backend, workload, status}` — never request ids or prompt
text. High-cardinality labels would blow up Prometheus and leak user data.

## 9. Mock backends are first-class

The mocks (`tests/mocks/`) let the entire control plane — routing, admission,
backpressure, breaker, streaming, failure handling — be tested and demoed on a
laptop, and gate every PR in CI. They are stdlib-only so CI needs no installs.
