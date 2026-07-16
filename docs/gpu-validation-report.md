# GPU validation report — 2026-07-15

## Qualification outcome

**BLOCKED / NR.** The available host is Apple M4 arm64 with no NVIDIA device,
NVIDIA driver, CUDA compiler, or active Docker daemon. The NVIDIA Container
Toolkit and a minimal CUDA container therefore could not be verified. Sanitized
details are in the [environment record](../artifacts/environment/gpu-validation-environment.md).

The validation protocol requires GPU phases to stop at this point. No mock
benchmark was rerun or relabeled as hardware progress.

## Validation matrix

| Capability | Mock | Real CPU | Single GPU | Multi-GPU | Multi-node |
|---|---|---|---|---|---|
| Gateway API | Pass | NR | NR | NR | NR |
| Triton | Pass | NR | NR | NR | NR |
| Dynamo/vLLM | Pass | N/A | NR | NR | NR |
| Streaming | Pass | N/A | NR | NR | NR |
| Fault recovery | Pass | NR | NR | NR | NR |

## Mandatory GPU gates

| Gate | Status | Reason |
|---|---|---|
| Real Triton ONNX inference | NR | No NVIDIA GPU or container runtime |
| FusionServe → real Triton | NR | Upstream service could not start |
| Real Dynamo frontend/router + vLLM | NR | No NVIDIA GPU or container runtime |
| Real streaming and non-streaming LLM | NR | Dynamo/vLLM could not start |
| Triton/Dynamo GPU benchmarks | NR | No real backend |
| Real-backend fault injection | NR | No real backend |
| DCGM/GPU dashboard data | NR | No NVIDIA device |
| TensorRT conversion | NR | No CUDA/TensorRT environment |

Single-GPU, multi-GPU, multi-worker, multi-node, disaggregated, and KV-aware
routing validation remain NR. The `v0.1.0` release gate is not satisfied.
