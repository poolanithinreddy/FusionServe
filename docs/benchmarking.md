# Benchmarking

## Reproducibility rule

Every measured run must state whether it used mocks or NVIDIA hardware and must
record the exact command, Git commit, date, OS, CPU, RAM, GPU, driver, CUDA,
Triton, Dynamo, vLLM, model, precision, and input/output distribution. Mock
numbers validate the gateway and harness only; they are never evidence of GPU
inference performance.

GPU performance numbers are published only when measured on real hardware with
a full environment record. CPU/mock observations may be retained as harness and
gateway regression evidence when they are unmistakably labeled as mock.

## Environment record (required in every report)

```
GPU:                 GPU memory:
CPU:                 RAM:
OS:                  NVIDIA driver:      CUDA:
Triton version:      Dynamo version:     vLLM version:
Model:               Precision:          Input dimensions:
Prompt distribution: Output-token distribution:
```

Generate the machine part with `bash scripts/verify_environment.sh`. Pin exact
versions — never `latest`.

## Tools

- **Non-LLM:** NVIDIA Triton [Performance Analyzer] (`perf_analyzer`) for the
  authoritative server-side numbers, plus `benchmarks/gateway/run_gateway_overhead.py`
  for the client-observed / gateway-overhead view.
- **LLM:** NVIDIA [GenAI-Perf] / Dynamo's documented benchmarking flow for the
  authoritative numbers, plus `benchmarks/llm/run_llm_bench.py` for client-side
  TTFT / inter-token latency through the gateway.

## Non-LLM matrix

| axis | values |
|------|--------|
| protocol | HTTP, gRPC |
| backend | ONNX Runtime, TensorRT |
| dynamic batching | off, moderate, throughput (see `models/triton/*/config.pbtxt`) |
| concurrency | 1, 2, 4, 8, 16, 32, 64 |
| instance count | 1, 2 |

Record: throughput, p50/p95/p99, GPU util, GPU memory, average realized batch
size, queue delay, error rate.

## LLM matrix

| axis | values |
|------|--------|
| workers | 1, 2 |
| routing | baseline, KV-aware |
| concurrency | 1, 2, 4, 8, 16 |
| prompt | unique, repeated-prefix |
| prompt length | short, medium, long |
| output length | 32, 128, 256 |
| streaming | on, off |

Record: req/s, tokens/s, TTFT, inter-token latency, e2e latency, p95/p99, GPU
memory/util, error rate.

## Gateway comparison and overhead

Run each driver twice — once at the gateway, once directly at the backend — and
report the two latency distributions side by side. Do not subtract independently
sampled percentiles: `p99(gateway) - p99(direct)` is not the p99 of an overhead
distribution and may even produce a non-monotonic result.

An actual overhead distribution requires paired observations under controlled
conditions. For pair `i`, calculate:

```
added_latency_i = gateway_latency_i - direct_backend_latency_i
```

Then compute percentiles from the `added_latency_i` sample. Record the pairing
strategy, alternate request order to reduce drift, use identical inputs, and
retain every raw pair. If paired data is unavailable, make no tail-overhead
claim; report only the independent distributions and throughput difference.

Every publishable distribution must satisfy
`p50 <= p90 <= p95 <= p99 <= max`. Validate result documents before publication:

```bash
python3 benchmarks/validate_results.py artifacts/benchmarks/mock
```

## Producing a report

Write each run's JSON (including an `environment` block) into
`benchmarks/results/`, then:

```bash
python3 benchmarks/generate_report.py --input benchmarks/results --out docs/benchmark-results.md
```

The generator flags any run missing an environment record as **UNVERIFIED** so
unmeasured numbers can never masquerade as measured.

## Claim discipline

- "Configured and benchmarked Triton dynamic batching" ✅
- "Implemented dynamic batching" ❌ (Triton implements it)
- "Integrated and benchmarked Dynamo KV-aware routing" ✅
- "Implemented KV-aware routing" ❌ (Dynamo implements it)

[Performance Analyzer]: https://github.com/triton-inference-server/perf_analyzer
[GenAI-Perf]: https://github.com/triton-inference-server/perf_analyzer/tree/main/genai-perf
