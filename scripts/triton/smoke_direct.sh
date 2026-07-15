#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
request="$(mktemp)"
response="$(mktemp)"
trap 'rm -f "$request" "$response"' EXIT
python3 "$ROOT/scripts/triton/make_request.py" >"$request"
curl --fail --silent --show-error \
  -H 'content-type: application/json' \
  --data-binary "@$request" \
  http://127.0.0.1:8000/v2/models/resnet50_onnx/infer >"$response"
python3 - "$response" <<'PY'
import json, sys
body = json.load(open(sys.argv[1]))
outputs = {item["name"]: item for item in body.get("outputs", [])}
assert "resnetv24_dense0_fwd" in outputs, body
assert outputs["resnetv24_dense0_fwd"]["shape"] == [1, 1000], body
print("direct Triton inference passed")
PY
