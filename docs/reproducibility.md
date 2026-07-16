# Reproducibility

## CPU/mock validation

The mock services are deterministic protocol fixtures. They let contributors
reproduce gateway correctness, cancellation, failure policy, metrics, and
benchmark-result validation without a GPU:

```bash
make build
python3 -m pytest clients/python/tests tests/integration -q
bash tests/failure/run_failure_suite.sh
python3 benchmarks/validate_results.py artifacts/benchmarks/mock
```

The retained CPU/mock measurements are historical observations from commit
`617ebe4`; this validation cycle did not rerun them as a substitute for GPU
progress.

## NVIDIA validation

First record a sanitized environment and verify `nvidia-smi`, Docker GPU access,
image compatibility, and available memory. Then follow
[Triton integration](triton-integration.md) and
[Dynamo/vLLM integration](dynamo-integration.md). Preserve raw command output,
image digests, model checksum/revision, requests, response summaries, repetitions,
and failure evidence under `artifacts/`.

This repository does not contain model weights. Downloaded models and generated
TensorRT engines must remain ignored. A release tag requires both real Triton
and real Dynamo/vLLM validation plus GPU benchmark evidence.
