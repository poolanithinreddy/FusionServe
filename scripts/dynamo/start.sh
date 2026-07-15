#!/usr/bin/env bash
set -euo pipefail

IMAGE="${FUSIONSERVE_DYNAMO_IMAGE:-nvcr.io/nvidia/ai-dynamo/vllm-runtime:1.2.0}"
MODEL="${FUSIONSERVE_DYNAMO_MODEL:-Qwen/Qwen3-0.6B}"
NAME="${FUSIONSERVE_DYNAMO_CONTAINER:-fusionserve-dynamo}"

docker info >/dev/null
nvidia-smi >/dev/null
docker run --detach --rm --gpus all --network host --name "$NAME" \
  -e HF_HOME=/workspace/.cache/huggingface \
  "$IMAGE" bash -lc \
  "python3 -m dynamo.frontend --http-port 8000 --discovery-backend file > /tmp/frontend.log 2>&1 & exec python3 -m dynamo.vllm --model '$MODEL' --discovery-backend file"

for _ in $(seq 1 300); do
  if curl --fail --silent http://127.0.0.1:8000/health >/dev/null; then
    echo "Dynamo frontend and vLLM worker are ready"
    exit 0
  fi
  sleep 1
done
docker logs "$NAME" >&2
echo "Dynamo readiness timed out" >&2
exit 1
