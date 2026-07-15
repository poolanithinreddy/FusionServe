#!/usr/bin/env bash
# Reproducible fault-injection suite against the mock stack (no GPU).
#
# Scenarios:
#   1. Triton unavailable  -> non-LLM fails fast / breaker opens; LLM unaffected.
#   2. Queue saturation     -> overload is shed with 429/503; gateway stays up.
#   3. Slow backend         -> requests bounded by deadline, not unbounded latency.
#
# Prints PASS/FAIL per scenario and exits non-zero if any scenario fails.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT" || exit 1

TRITON_PORT=18701
DYNAMO_PORT=18700
GW="http://127.0.0.1:18780"
CFG="tests/failure/failure_config.yaml"
FAILED=0
PIDS=()

# Called indirectly by the EXIT/INT/TERM trap below.
# shellcheck disable=SC2329
cleanup() {
  for p in "${PIDS[@]:-}"; do kill "$p" 2>/dev/null; done
  wait 2>/dev/null
}
trap cleanup EXIT

wait_ready() {
  for _ in $(seq 1 50); do
    curl -sf "$GW/healthz" >/dev/null 2>&1 && return 0
    sleep 0.2
  done
  return 1
}

echo "== building gateway =="
cargo build -p fusionserve-gateway --quiet || { echo "build failed"; exit 1; }
BIN="$ROOT/target/debug/fusionserve-gateway"

echo "== starting mocks + gateway =="
python3 tests/mocks/mock_triton.py --port "$TRITON_PORT" >/tmp/fl_triton.log 2>&1 & PIDS+=($!)
python3 tests/mocks/mock_dynamo.py --port "$DYNAMO_PORT" >/tmp/fl_dynamo.log 2>&1 & PIDS+=($!)
TRITON_PID=${PIDS[0]}
FUSIONSERVE_CONFIG="$CFG" RUST_LOG=error "$BIN" >/tmp/fl_gw.log 2>&1 & PIDS+=($!)
wait_ready || { echo "gateway did not start"; cat /tmp/fl_gw.log; exit 1; }
echo "ready"

# --- Scenario 1: Triton unavailable ---------------------------------------
echo; echo "== Scenario 1: Triton unavailable =="
kill "$TRITON_PID" 2>/dev/null; sleep 0.3
# Several infer calls: expect fast failures (503), and after threshold the
# breaker should be open. LLM chat should still succeed.
codes=""
for _ in $(seq 1 6); do
  c=$(curl -s -o /dev/null -w "%{http_code}" -m 3 -X POST "$GW/v1/infer/resnet50" \
        -H 'content-type: application/json' -d '{"inputs":[1,2,3]}')
  codes="$codes $c"
done
echo "infer codes:$codes"
chat=$(curl -s -o /dev/null -w "%{http_code}" -m 5 -X POST "$GW/v1/chat/completions" \
        -H 'content-type: application/json' -d '{"model":"qwen_small","messages":[{"role":"user","content":"hi"}]}')
echo "chat code: $chat"
if [[ "$codes" == *"503"* && "$chat" == "200" ]]; then
  echo "PASS: non-LLM fails fast while LLM stays healthy"
else
  echo "FAIL: expected 503 on infer and 200 on chat"; FAILED=1
fi

# Restart triton for remaining scenarios.
python3 tests/mocks/mock_triton.py --port "$TRITON_PORT" >/tmp/fl_triton2.log 2>&1 & PIDS+=($!)
sleep 1

# --- Scenario 2: Queue saturation -----------------------------------------
echo; echo "== Scenario 2: Queue saturation =="
# Make Triton slow so requests occupy the single concurrency slot.
kill "${PIDS[-1]}" 2>/dev/null; sleep 0.2
MOCK_LATENCY_MS=800 python3 tests/mocks/mock_triton.py --port "$TRITON_PORT" >/tmp/fl_triton3.log 2>&1 & PIDS+=($!)
sleep 0.5
if python3 tests/failure/saturate_queue.py "$GW" 40; then
  echo "PASS: overload shed gracefully"
else
  echo "FAIL: queue saturation not handled"; FAILED=1
fi

# --- Scenario 3: Slow backend / deadline ----------------------------------
echo; echo "== Scenario 3: Slow backend deadline bound =="
# A single request against the 800ms-latency backend with a 1000ms model timeout
# should return within a few seconds (not hang). Measure wall time.
start=$(date +%s%N)
code=$(curl -s -o /dev/null -w "%{http_code}" -m 5 -X POST "$GW/v1/infer/resnet50" \
        -H 'content-type: application/json' -d '{"inputs":[1,2,3]}')
end=$(date +%s%N)
ms=$(( (end - start) / 1000000 ))
echo "single request: code=$code elapsed=${ms}ms"
if (( ms < 4000 )); then
  echo "PASS: request bounded by deadline (${ms}ms)"
else
  echo "FAIL: request exceeded expected bound"; FAILED=1
fi

# --- Scenario 4: Dynamo unavailable; Triton remains functional -------------
echo; echo "== Scenario 4: Dynamo unavailable =="
kill "${PIDS[1]}" 2>/dev/null; sleep 0.3
chat=$(curl -s -o /dev/null -w "%{http_code}" -m 3 -X POST "$GW/v1/chat/completions" \
        -H 'content-type: application/json' -d '{"model":"qwen_small","messages":[{"role":"user","content":"hi"}]}')
infer=$(curl -s -o /dev/null -w "%{http_code}" -m 3 -X POST "$GW/v1/infer/resnet50" \
        -H 'content-type: application/json' -d '{"inputs":[1,2,3]}')
echo "chat code=$chat infer code=$infer"
if [[ "$chat" == "503" && "$infer" == "200" ]]; then
  echo "PASS: LLM fails fast while non-LLM remains healthy"
else
  echo "FAIL: expected chat=503 and infer=200"; FAILED=1
fi

echo
if (( FAILED == 0 )); then
  echo "ALL FAILURE SCENARIOS PASSED"
else
  echo "SOME FAILURE SCENARIOS FAILED"
fi
exit $FAILED
