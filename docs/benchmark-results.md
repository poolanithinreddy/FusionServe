# FusionServe Benchmark Results

## 2026-07-15 local CPU/mock validation

These measurements exercise the real Rust gateway against the repository's
Python mock Triton and mock Dynamo services. They validate routing, admission,
streaming, observability, and the benchmark harness. They do **not** measure
Triton, Dynamo, vLLM, CUDA, model execution, GPU utilization, or GPU memory.

Environment:

- Git commit under test: `617ebe4` (before the final documentation/tooling update)
- Machine: Apple M4 MacBook Air, 10 cores (4 performance, 6 efficiency)
- Memory: 16 GB
- OS: macOS 26.5.1, Darwin 25.5.0, arm64
- Rust: 1.94.1
- Python: 3.9.6
- Triton/Dynamo/vLLM/CUDA/GPU model: not used; repository mocks only

### Non-LLM gateway path

Command:

```bash
python3 benchmarks/gateway/run_gateway_overhead.py \
  --target http://127.0.0.1:8080 --path /v1/infer \
  --body '{"model":"resnet50","inputs":[0.1,0.2,0.3]}' \
  --requests 1000 --concurrency 8 --warmup 50
```

| Path | Success/error | Throughput | p50 | p95 | p99 | Mean |
|---|---:|---:|---:|---:|---:|---:|
| FusionServe -> mock Triton | 1000 / 0 | 4,883.45 req/s | 1.470 ms | 2.365 ms | 2.841 ms | 1.563 ms |
| Direct mock Triton | 1000 / 0 | 6,038.42 req/s | 1.170 ms | 1.898 ms | 2.407 ms | 1.229 ms |

Observed gateway overhead was 0.300 ms at p50, 0.467 ms at p95, and 0.434 ms
at p99. This subtraction is useful for local regression tracking; it is not a
claim about overhead with real Triton inference.

### Streaming LLM gateway path

Command:

```bash
python3 benchmarks/llm/run_llm_bench.py \
  --target http://127.0.0.1:8080 \
  --requests 128 --concurrency 8 --warmup 8
```

| Success/error | Requests/s | Mock tokens/s | TTFT p50/p95/p99 | E2E p50/p95/p99 |
|---:|---:|---:|---:|---:|
| 128 / 0 | 101.77 | 915.91 | 27.56 / 30.11 / 31.27 ms | 77.29 / 80.37 / 82.04 ms |

The mock intentionally sleeps 20 ms before its first token and 5 ms between
tokens. These results therefore test streaming forwarding and measurement
correctness rather than language-model performance.

## GPU results

No compatible NVIDIA GPU validation was run. Real Triton dynamic-batching,
Dynamo KV-aware-routing, vLLM token throughput, CUDA behavior, GPU utilization,
and GPU memory results remain unverified and must not be inferred from this page.
