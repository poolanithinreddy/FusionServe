#!/usr/bin/env bash
set -euo pipefail
response="$(mktemp)"
headers="$(mktemp)"
trap 'rm -f "$response" "$headers"' EXIT
curl --fail --silent --show-error --dump-header "$headers" \
  http://127.0.0.1:8080/v1/chat/completions \
  -H 'content-type: application/json' -H 'x-request-id: dynamo-smoke-1' \
  -d '{"model":"Qwen/Qwen3-0.6B","messages":[{"role":"user","content":"Reply with OK"}],"max_tokens":8,"temperature":0}' \
  >"$response"
grep -qi '^x-request-id: dynamo-smoke-1' "$headers"
python3 - "$response" <<'PY'
import json, sys
body = json.load(open(sys.argv[1]))
assert body.get("choices"), body
print("FusionServe to Dynamo/vLLM chat passed")
PY
