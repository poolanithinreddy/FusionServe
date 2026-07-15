#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="${FUSIONSERVE_TRITON_RESULTS:-$ROOT/artifacts/benchmarks/triton}"
SDK_IMAGE="${FUSIONSERVE_TRITON_SDK_IMAGE:-nvcr.io/nvidia/tritonserver:26.04-py3-sdk}"
mkdir -p "$OUT"

docker info >/dev/null
nvidia-smi >/dev/null

for protocol in http grpc; do
  if [[ "$protocol" == http ]]; then port=8000; else port=8001; fi
  for concurrency in 1 8 16 32 64; do
    output="$OUT/perf_${protocol}_c${concurrency}.csv"
    docker run --rm --network host -v "$OUT:/results" "$SDK_IMAGE" perf_analyzer \
      -m resnet50_onnx -u "127.0.0.1:${port}" -i "$protocol" \
      --concurrency-range "${concurrency}:${concurrency}" \
      --shape data:1,3,224,224 --input-data zero \
      --measurement-mode count_windows --measurement-request-count 200 \
      -f "/results/$(basename "$output")"
  done
done

python3 "$ROOT/benchmarks/triton/summarize.py" "$OUT"/*.csv \
  --output "$OUT/summary.json"
