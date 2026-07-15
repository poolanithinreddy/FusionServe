# Limitations & Verification Status

This document states honestly what is and is not verified, so every claim can be
traced to evidence. States follow the project's convention:
**VERIFIED / PARTIALLY VERIFIED / UNVERIFIED**.

## What is VERIFIED (reproducible in this repo, no GPU)

- **Gateway builds cleanly**: `cargo build`, `cargo clippy -D warnings`. VERIFIED.
- **Unit + integration tests pass**: 11 unit tests (circuit breaker, backpressure,
  classifier) + 9 gateway integration tests (error model, discovery, admission
  pre-checks, admin, metrics). VERIFIED — `cargo test`.
- **End-to-end request path** through the real binary + mock backends: non-LLM →
  Triton path, LLM unary + **streaming** → Dynamo path, request-id propagation,
  error contract. VERIFIED — `pytest tests/integration/test_end_to_end.py` (6/6).
- **Python client** drives all paths against the live stack. VERIFIED.
- **C++ client** compiles + links (clang++ + libcurl) and runs end-to-end against
  the live stack (classify, unary chat, streaming). VERIFIED in this environment.
  Note: the GoogleTest suite is written but was not executed here because CMake was
  not installed; the CI job (`cpp-ci`) builds and runs it.
- **Failure behavior**: Triton-down isolation, queue-saturation load-shedding
  (429), and slow-backend deadline bounding. VERIFIED — `tests/failure/run_failure_suite.sh`.
- **Metrics** exposed in Prometheus format with bounded cardinality. VERIFIED.
- **All YAML** (compose, k8s, config) parses. VERIFIED.

## What is UNVERIFIED (requires NVIDIA GPU + real engines)

- **Real Triton / Dynamo / vLLM integration.** The clients speak the correct
  protocols (KServe v2, OpenAI-compatible) and are verified against mocks, but
  have **not** been run against the real servers in this environment. UNVERIFIED.
- **All performance numbers.** No throughput/latency/TTFT/GPU numbers are
  published; `docs/benchmark-results.md` is intentionally empty until measured on
  hardware with a full environment record. UNVERIFIED.
- **TensorRT engine build** (`build_tensorrt_engine.sh`) — needs `trtexec` + GPU.
- **Kubernetes manifests** — authored and YAML-valid, but not applied to a live
  cluster here. UNVERIFIED at runtime.
- **Model download URLs** — the ResNet-50 ONNX URL is a best-effort default;
  `text_embedding` requires you to export/point at a model. Verify before use.

## Scope / wording discipline (claims NOT made)

- **Not** "implemented Triton dynamic batching" — it is **configured and
  (to be) benchmarked**. Triton implements batching.
- **Not** "implemented Dynamo KV-aware routing" — it is **integrated and (to be)
  benchmarked**. Dynamo implements routing.
- **Not** "Rust/C++ control plane" — C++ is a **client**; the control plane is Rust.
- **Not** "multi-node inference" — only local containers/mocks are exercised here.
- **Not** "CUDA programming" — no CUDA kernels are written.
- **Not** "production-ready" — no external security/reliability sign-off.
- **Not** "contributed to Triton/Dynamo" — no upstream PR/issue exists.

## Known gaps / future work

- gRPC transport to Triton (`protocol: grpc` is modeled but the client is
  HTTP-only today).
- Per-endpoint (vs per-model) admission sharing when multiple models hit one
  backend — currently admission is per model, health is per endpoint.
- Distributed tracing spans are structured via `tracing`; OpenTelemetry export
  is wired conceptually but not shipped to a collector in this repo.
- GPU metrics (DCGM/NVML) are documented but collected from Triton/exporters, not
  the gateway.
