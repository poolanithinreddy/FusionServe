#!/usr/bin/env bash
set -euo pipefail
NAME="${FUSIONSERVE_DYNAMO_CONTAINER:-fusionserve-dynamo}"
docker stop "$NAME"
