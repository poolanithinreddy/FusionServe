# Deployment

Two stacks: the **mock** stack (no GPU, for development/CI) and the **GPU** stack
(real Triton + Dynamo + vLLM).

## Mock stack (no GPU)

```bash
# Option A: Docker
make dev-up          # gateway + mock Triton + mock Dynamo + Prometheus + Grafana
make dev-down

# Option B: bare processes
python3 tests/mocks/mock_triton.py --port 8001 &
python3 tests/mocks/mock_dynamo.py --port 8000 &
cd gateway-rs && cargo run
```

Grafana is at `http://localhost:3000` (anonymous admin), Prometheus at `:9090`.
Import `dashboards/fusionserve-grafana.json`.

## GPU stack

Prerequisites: NVIDIA GPU, recent driver, [NVIDIA Container Toolkit], Docker.

1. **Get models** (nothing is committed):
   ```bash
   python3 scripts/download_models.py --write-checksums
   bash scripts/verify_model_checksums.sh
   # Optional TensorRT engine (run inside the Triton container):
   PRECISION=fp16 bash scripts/build_tensorrt_engine.sh
   ```
2. **Record the environment** (required for any benchmark):
   ```bash
   bash scripts/verify_environment.sh
   ```
3. **Bring it up**:
   ```bash
   make gpu-up      # docker compose --profile gpu
   ```

### Version pinning

Pin exact image tags before benchmarking — do not use `latest`. As referenced in
the project brief, Dynamo's repo showed release `1.2.1` and Triton's example used
the `26.06` line; **recheck the support matrix** for compatible Triton / Dynamo /
vLLM / CUDA / driver versions before a final run. Record the versions you used in
the benchmark environment block.

## Dynamo topology

The gateway sends LLM traffic to the **Dynamo frontend**, which owns routing to
vLLM workers. Do not bypass it. For multi-worker experiments (baseline vs.
KV-aware routing, worker-failure tests), scale the vLLM worker Deployment and let
Dynamo's router distribute — see the [Dynamo docs][dynamo]. FusionServe measures
these; it does not implement them.

## Kubernetes

```bash
kubectl apply -f deploy/kubernetes/namespace.yaml
kubectl apply -f deploy/kubernetes/           # gateway, triton, dynamo, prom, grafana
```

The gateway Deployment sets readiness (`/readyz`) and liveness (`/healthz`)
probes, resource requests/limits, non-root securityContext, and Prometheus
scrape annotations. Triton/Dynamo require GPU nodes (`nvidia.com/gpu: 1`) and a
model-repository volume you provision.

## Security notes

- Admin endpoints (`/admin/*`) mutate routing — put them behind auth / network
  policy before any non-local exposure.
- No secrets are read or stored by the gateway; configuration is non-secret.
- Run the container as non-root (the image already does).

[NVIDIA Container Toolkit]: https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/latest/index.html
[dynamo]: https://github.com/ai-dynamo/dynamo
