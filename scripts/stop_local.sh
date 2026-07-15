#!/usr/bin/env bash
set -uo pipefail
for f in /tmp/fs_gw.pid /tmp/fs_triton.pid /tmp/fs_dynamo.pid; do
  [[ -f "$f" ]] && kill "$(cat "$f")" 2>/dev/null && rm -f "$f"
done
echo "stopped local stack"
