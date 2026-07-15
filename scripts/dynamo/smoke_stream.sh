#!/usr/bin/env bash
set -euo pipefail
output="$(mktemp)"
trap 'rm -f "$output"' EXIT
curl --fail --silent --show-error --no-buffer \
  http://127.0.0.1:8080/v1/chat/completions \
  -H 'content-type: application/json' \
  -d '{"model":"Qwen/Qwen3-0.6B","stream":true,"messages":[{"role":"user","content":"Count to three"}],"max_tokens":16}' \
  >"$output"
grep -q '^data:' "$output"
grep -q 'data: \[DONE\]' "$output"
echo "FusionServe streaming through Dynamo/vLLM passed"
