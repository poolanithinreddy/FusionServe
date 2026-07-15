# Dynamo/vLLM benchmark artifacts

Status: **NOT RUN** on the current CPU-only Apple Silicon environment.

The pinned validation path uses Dynamo vLLM runtime `1.2.0` with
`Qwen/Qwen3-0.6B`. It requires an NVIDIA GPU, a compatible driver, NVIDIA
Container Toolkit, a running Docker daemon, and sufficient model-cache disk.
No single-GPU, multi-worker, distributed, KV-aware-routing, or GPU performance
claim is supported until raw results are written here.
