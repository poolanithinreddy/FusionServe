#!/usr/bin/env bash
set -euo pipefail
response="$(mktemp)"
trap 'rm -f "$response"' EXIT
curl --fail --silent --show-error http://127.0.0.1:8000/v1/chat/completions \
  -H 'content-type: application/json' \
  -d '{"model":"Qwen/Qwen3-0.6B","messages":[{"role":"user","content":"Reply with OK"}],"max_tokens":8,"temperature":0}' \
  >"$response"
python3 - "$response" <<'PY'
import json, sys
body = json.load(open(sys.argv[1]))
assert body.get("choices"), body
assert body["choices"][0].get("message", {}).get("content") is not None, body
print("direct Dynamo/vLLM chat passed")
PY
