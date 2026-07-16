# Validation report — 2026-07-15

## Environment

Classification: **CPU-only**. Apple M4, 10 CPU cores, 16 GB RAM, macOS arm64.
No NVIDIA GPU, NVIDIA driver, CUDA runtime, or NVIDIA Container Toolkit was
available. Docker CLI/Compose were installed; the Docker daemon was not running.
See the sanitized [GPU qualification environment](../artifacts/environment/gpu-validation-environment.md).

## Verified locally

- Rust formatting and Clippy with warnings denied.
- 45 Rust tests (28 unit + 17 API), including live in-process Triton/Dynamo
  protocol fixtures for unary, streaming, timeout, status, malformed-response,
  and request-ID paths.
- 32 Python client/integration tests, including deadline, cancellation,
  malformed-response, circuit recovery, and Triton contract tests.
- Four backend/failure-process scenarios and ten total fault classes.
- Mock and GPU Compose configuration parsing.
- Ruff formatting/lint and mypy typing.
- ShellCheck and shell syntax validation.
- CMake 4.4 C++17/libcurl build and 10/10 CTest tests.
- RustSec audit of 235 locked dependencies with no known vulnerability.
- Rust line coverage: 80.65%; nightly LLVM branch coverage: 61.54%.
- CPU/mock benchmark raw JSON and documentation.
- Benchmark-result integrity and repository-relative documentation links.

## Not run

- Real Triton container/model loading/inference or Performance Analyzer.
- Real Dynamo frontend/router/vLLM worker or LLM benchmark.
- CUDA, TensorRT, DCGM, GPU utilization, or GPU memory measurements.
- Single-GPU, multi-worker, multi-GPU, disaggregated, or KV-aware-routing tests.

The blockers are hardware/runtime availability, not hidden test failures. The
repository contains exact pinned workflows for a compatible NVIDIA runner.
See the [GPU validation report](gpu-validation-report.md) and sanitized
[environment record](../artifacts/environment/gpu-validation-environment.md).

## Claim boundary

FusionServe implements gateway policy and integrates upstream request protocols.
Triton owns model execution, scheduling, and dynamic batching. Dynamo owns worker
discovery and KV-aware routing. A proposal-only Triton tutorials documentation
correction is recorded in [the upstream candidate](upstream-contribution-candidate.md);
no external issue, fork, or pull request was created.
