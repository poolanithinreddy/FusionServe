#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
[[ -f "$ROOT/models/triton/resnet50_onnx/1/model.onnx" ]] || {
  echo "model missing; run scripts/triton/download_resnet50.sh" >&2
  exit 1
}
docker info >/dev/null
docker compose -f "$ROOT/deploy/triton/docker-compose.yml" up -d
"$ROOT/scripts/triton/wait_ready.sh"
