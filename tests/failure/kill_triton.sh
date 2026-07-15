#!/usr/bin/env bash
# Kill the Triton (or mock-triton) process to simulate backend outage.
set -uo pipefail
pkill -f 'mock_triton.py' && echo "killed mock triton" || echo "no mock triton running"
