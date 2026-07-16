# NVIDIA Triton integration

Status on the recorded environment: **NOT RUN**. The host is Apple Silicon with
no NVIDIA GPU and Docker Desktop's daemon was unavailable.
This remains NR in the [GPU validation report](gpu-validation-report.md).

The workflow pins NVIDIA Triton `26.04`, matching the official server release
examples, and downloads ONNX Model Zoo ResNet-50 v2 with SHA-256 verification.
The checked-in `config.pbtxt` uses the graph's actual `data` input and
`resnetv24_dense0_fwd` output. The model binary remains ignored by Git.

Prerequisites: Linux NVIDIA host, compatible driver, NVIDIA Container Toolkit,
Docker with `--gpus all`, and at least 2 GiB free disk.

```bash
scripts/triton/download_resnet50.sh
scripts/triton/start.sh
scripts/triton/smoke_direct.sh

# In another terminal, run the native gateway:
cargo run -p fusionserve-gateway -- --config deploy/triton/fusionserve.yaml
scripts/triton/smoke_fusionserve.sh

# HTTP/gRPC concurrency matrix via Performance Analyzer:
benchmarks/triton/run_matrix.sh
scripts/triton/stop.sh
```

Raw CSV and normalized JSON belong in `artifacts/benchmarks/triton/`. The fixed
batch-one ONNX graph is a baseline; dynamic batching must not be claimed until a
genuinely dynamic-batch model is exported, loaded, and measured. TensorRT is
also unverified until engine conversion succeeds on the target GPU.

Official references: [Triton model repository](https://docs.nvidia.com/deeplearning/triton-inference-server/user-guide/docs/user_guide/model_repository.html)
and [ONNX deployment quick start](https://docs.nvidia.com/deeplearning/triton-inference-server/user-guide/docs/tutorials/Quick_Deploy/ONNX/README.html).
