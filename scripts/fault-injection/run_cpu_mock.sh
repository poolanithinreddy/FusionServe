#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

bash tests/failure/run_failure_suite.sh
python3 -m pytest tests/integration/test_end_to_end.py -q

echo "CPU/mock fault-injection validation passed."
echo "Evidence: artifacts/fault-injection/cpu-mock-validation.md"
