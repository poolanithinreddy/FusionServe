# FusionServe

FusionServe is a systems-software project that provides a unified inference
gateway for heterogeneous AI workloads. It routes non-LLM inference to NVIDIA
Triton and LLM requests to NVIDIA Dynamo with vLLM while adding shared admission
control, backpressure, failure handling, observability, and client libraries.

The project focuses on the engineering trade-offs involved in production
inference serving:

- throughput versus tail latency
- batching versus queue delay
- overload protection
- request cancellation
- GPU memory visibility
- distributed worker failure
- consistent APIs across inference engines
- reproducible performance measurement

## Architecture

```text
                          +--------------------------+
                          |   Python / C++ Clients   |
                          +------------+-------------+
                                       |
                                 HTTP / gRPC
                                       |
                      +----------------v----------------+
                      |      FusionServe Gateway        |
                      |             (Rust)              |
                      | classify · registry · admission |
                      | bounded queues · backpressure   |
                      | timeouts · cancellation         |
                      | circuit breakers · health route |
                      | unified tracing / metrics        |
                      +---------+--------------+--------+
                Non-LLM path |              | LLM path
                 +-----------v----+     +---v----------------+
                 | NVIDIA Triton  |     | NVIDIA Dynamo       |
                 | ONNX/TensorRT  |     | frontend + router   |
                 | dynamic batch  |     |    |       |        |
                 +-------+--------+     | vLLM-A   vLLM-B     |
                         |              +--------------------+
                     NVIDIA GPU
        +-----------------------------------------------+
        | Prometheus + Grafana + OpenTelemetry           |
        +-----------------------------------------------+
```

The gateway owns cross-cutting reliability concerns. Triton owns non-LLM model
execution and dynamic batching; Dynamo owns LLM worker discovery and KV-aware
routing. FusionServe **integrates and benchmarks** those engine capabilities —
it does not reimplement them. See [docs/design-decisions.md](docs/design-decisions.md)
and [docs/limitations.md](docs/limitations.md) for the exact division of
ownership and honest scope.

## Quick start (no GPU required)

The default development stack runs the Rust gateway against **mock** Triton and
Dynamo backends, so you can exercise routing, admission control, backpressure,
circuit breaking, and observability on a laptop.

```bash
# 1. Build and start the mock stack (gateway + mock Triton + mock Dynamo)
make dev-up

# 2. Health
curl localhost:8080/healthz
curl localhost:8080/readyz

# 3. Non-LLM inference (routed to mock Triton)
curl -s localhost:8080/v1/infer/resnet50 \
  -H 'content-type: application/json' \
  -d '{"inputs":[0.1,0.2,0.3]}' | jq

# 4. LLM chat (routed to mock Dynamo, OpenAI-compatible)
curl -s localhost:8080/v1/chat/completions \
  -H 'content-type: application/json' \
  -d '{"model":"qwen_small","messages":[{"role":"user","content":"hi"}]}' | jq

# 5. Metrics
curl -s localhost:8080/metrics | grep fusionserve_

make dev-down
```

Run the whole thing without Docker:

```bash
# terminal 1
python3 tests/mocks/mock_triton.py --port 8001
# terminal 2
python3 tests/mocks/mock_dynamo.py --port 8000
# terminal 3
cd gateway-rs && cargo run
```

## GPU stack

Bringing up the real GPU stack (Triton + Dynamo + vLLM) requires an NVIDIA GPU
and the NVIDIA container toolkit. See [docs/deployment.md](docs/deployment.md).
Models are downloaded through scripts — **no model binaries are committed**.

```bash
python3 scripts/download_models.py        # fetch ONNX / LLM weights
make gpu-up                               # docker-compose with GPU profile
```

## Repository layout

| Path | What it is |
|------|-----------|
| [gateway-rs/](gateway-rs/) | The Rust gateway — the principal original systems component |
| [clients/python/](clients/python/) | Async Python client + benchmark harness |
| [clients/cpp/](clients/cpp/) | C++17 gRPC/HTTP client (GoogleTest) |
| [models/triton/](models/triton/) | Triton model repository (configs only) |
| [tests/mocks/](tests/mocks/) | Mock Triton & Dynamo for GPU-free testing |
| [tests/failure/](tests/failure/) | Fault-injection suite |
| [benchmarks/](benchmarks/) | Benchmark harness + report generator |
| [deploy/](deploy/) | Docker + Kubernetes manifests |
| [docs/](docs/) | Architecture, API, deployment, benchmarking, limitations |
| [.claude/](.claude/) | Feature-reference notes used while building this project |

## Implementation status

| Milestone | Status |
|-----------|--------|
| M1 Triton baseline (configs + mock + client) | Scaffolded (real GPU run: UNVERIFIED — needs hardware) |
| M2 Dynamo baseline (mock + OpenAI-compatible chat) | Scaffolded (real GPU run: UNVERIFIED) |
| M3 Rust unified gateway (registry, routing, error model, request-id) | **Implemented & tested against mocks** |
| M4 Production behavior (queues, limits, backpressure, timeouts, cancel, breaker, health routing) | **Implemented & tested against mocks** |
| M5 C++ client | **Compiles, links, and runs end-to-end** (clang++/libcurl) against the live mock stack; GoogleTest suite written, runs in CI |
| M6 Observability (Prometheus metrics, tracing, JSON logs) | **Implemented**; Grafana dashboard provided |
| M7 Performance study | Harness provided; **numbers UNVERIFIED — require GPU** |
| M8 Kubernetes + failure testing | Manifests + fault-injection scripts provided |

See [docs/limitations.md](docs/limitations.md) for exactly what has and has not
been verified. No benchmark numbers are published until they are measured on
real hardware and recorded with a full environment record.

## Development

```bash
make build        # cargo build (gateway)
make test         # cargo test + python tests against mocks
make lint         # cargo fmt --check, clippy -D warnings, ruff, mypy
make bench-mock   # run the benchmark harness against mock backends
```

## License

Apache-2.0. See [LICENSE](LICENSE). FusionServe is not affiliated with NVIDIA.
