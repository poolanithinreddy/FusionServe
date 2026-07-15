#!/usr/bin/env bash
# Kill the Dynamo frontend/worker (or mock) to simulate LLM worker loss.
set -uo pipefail
pkill -f 'mock_dynamo.py' && echo "killed mock dynamo" || echo "no mock dynamo running"
