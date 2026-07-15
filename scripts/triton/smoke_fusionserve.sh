#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
request="$(mktemp)"
response="$(mktemp)"
headers="$(mktemp)"
trap 'rm -f "$request" "$response" "$headers"' EXIT
python3 "$ROOT/scripts/triton/make_request.py" gateway >"$request"
curl --fail --silent --show-error --dump-header "$headers" \
  -H 'content-type: application/json' -H 'x-request-id: triton-smoke-1' \
  --data-binary "@$request" http://127.0.0.1:8080/v1/infer >"$response"
grep -qi '^x-request-id: triton-smoke-1' "$headers"
python3 - "$response" <<'PY'
import json, sys
body = json.load(open(sys.argv[1]))
assert any(item["name"] == "resnetv24_dense0_fwd" for item in body.get("outputs", [])), body
print("FusionServe to Triton inference passed")
PY
