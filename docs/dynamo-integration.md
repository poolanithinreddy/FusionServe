# NVIDIA Dynamo and vLLM integration

Status on the recorded environment: **NOT RUN**. No NVIDIA GPU, driver, CUDA
runtime, or active Docker daemon was available.

The reproduction workflow pins `nvcr.io/nvidia/ai-dynamo/vllm-runtime:1.2.0`
and the public `Qwen/Qwen3-0.6B` model. It starts Dynamo's OpenAI-compatible
frontend/router and a Dynamo-managed vLLM worker with file discovery. FusionServe
targets the frontend at port 8000; it does not bypass or replace Dynamo routing.

```bash
scripts/dynamo/start.sh
scripts/dynamo/smoke_direct.sh

# In another terminal:
cargo run -p fusionserve-gateway -- --config deploy/dynamo/fusionserve.yaml
scripts/dynamo/smoke_fusionserve.sh
scripts/dynamo/smoke_stream.sh
scripts/dynamo/stop.sh
```

This is a single-worker/single-GPU recipe. It cannot establish distributed,
multi-worker, multi-GPU, disaggregated, or KV-aware-routing performance claims.
Those require the official multi-worker recipes and raw traffic evidence on at
least two GPUs.

Official references: [Dynamo quick start](https://docs.nvidia.com/dynamo) and
[Dynamo vLLM backend](https://docs.nvidia.com/dynamo/dev/backends/v-llm).
