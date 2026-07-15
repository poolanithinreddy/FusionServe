# Validation report — 2026-07-15

## Environment

Classification: **CPU-only**. Apple M4, 10 CPU cores, 16 GB RAM, macOS arm64.
No NVIDIA GPU, NVIDIA driver, CUDA runtime, or NVIDIA Container Toolkit was
available. Docker CLI/Compose were installed; the Docker daemon was not running.
See the [environment artifact](../artifacts/environment/environment.md).

## Verified locally

- Rust formatting and Clippy with warnings denied.
- 39 Rust tests (28 unit + 11 API).
- Python client and integration suites, including deadline, cancellation,
  malformed-response, circuit recovery, and Triton contract tests.
- Four backend/failure-process scenarios and ten total fault classes.
- Mock and GPU Compose configuration parsing.
- Shell syntax and C++17/libcurl compilation.
- Rust line coverage: 57.24%.
- CPU/mock benchmark raw JSON and documentation.

## Not run

- Real Triton container/model loading/inference or Performance Analyzer.
- Real Dynamo frontend/router/vLLM worker or LLM benchmark.
- CUDA, TensorRT, DCGM, GPU utilization, or GPU memory measurements.
- Single-GPU, multi-worker, multi-GPU, disaggregated, or KV-aware-routing tests.

The blockers are hardware/runtime availability, not hidden test failures. The
repository contains exact pinned workflows for a compatible NVIDIA runner.

## Claim boundary

FusionServe implements gateway policy and integrates upstream request protocols.
Triton owns model execution, scheduling, and dynamic batching. Dynamo owns worker
discovery and KV-aware routing. No upstream contribution candidate was confirmed
during this CPU-only validation cycle.
