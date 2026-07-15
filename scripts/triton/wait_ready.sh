#!/usr/bin/env bash
set -euo pipefail
for _ in $(seq 1 120); do
  if curl --fail --silent http://127.0.0.1:8000/v2/health/ready >/dev/null && \
     curl --fail --silent http://127.0.0.1:8000/v2/models/resnet50_onnx/ready >/dev/null; then
    echo "Triton and resnet50_onnx are ready"
    exit 0
  fi
  sleep 1
done
echo "Triton readiness timed out" >&2
exit 1
