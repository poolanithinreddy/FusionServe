#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
docker compose -f "$ROOT/deploy/triton/docker-compose.yml" down
