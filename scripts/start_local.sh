#!/usr/bin/env bash
# Start the mock stack locally (no Docker, no GPU): mocks + gateway.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
python3 tests/mocks/mock_triton.py --port 8001 & echo $! > /tmp/fs_triton.pid
python3 tests/mocks/mock_dynamo.py --port 8000 & echo $! > /tmp/fs_dynamo.pid
( cargo build --offline -p fusionserve-gateway && ./target/debug/fusionserve-gateway --config gateway-rs/config.yaml ) &
echo $! > /tmp/fs_gw.pid
echo "gateway on :8080, mocks on :8001/:8000. Use scripts/stop_local.sh to stop."
wait
