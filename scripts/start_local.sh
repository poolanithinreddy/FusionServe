#!/usr/bin/env bash
# Start the mock stack locally (no Docker, no GPU): mocks + gateway.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
python3 tests/mocks/mock_triton.py --port 8001 & echo $! > /tmp/fs_triton.pid
python3 tests/mocks/mock_dynamo.py --port 8000 & echo $! > /tmp/fs_dynamo.pid
( cd gateway-rs && cargo build && FUSIONSERVE_CONFIG=config.yaml ./target/debug/fusionserve-gateway ) &
echo $! > /tmp/fs_gw.pid
echo "gateway on :8080, mocks on :8001/:8000. Use scripts/stop_local.sh to stop."
wait
