# Failure Testing

The fault-injection suite proves the gateway degrades gracefully instead of
crashing or hanging. It runs entirely against the mock stack (no GPU) and is
part of `integration-ci`.

```bash
bash tests/failure/run_failure_suite.sh
```

## Scenarios & expected behavior

| Scenario | Injection | Expected | Verified |
|----------|-----------|----------|----------|
| Triton unavailable | kill mock Triton | non-LLM returns 503 fast; circuit breaker opens; **LLM traffic unaffected** | ✅ (`infer codes: 503×6`, `chat: 200`) |
| Queue saturation | 40 concurrent calls, `max_concurrency=1`, slow backend | most requests shed with **429**; gateway stays alive | ✅ (`{429: 39, 200: 1}`, alive) |
| Slow backend | 800 ms backend latency, 1 s model timeout | single request **bounded** (~0.8 s), never hangs | ✅ (`elapsed=828ms`) |

These results were observed on the mock stack in this repo's environment; rerun
the suite to reproduce. The numbers above are illustrative of behavior, not
performance benchmarks.

## Additional scenarios (documented; run on the GPU stack)

- **Dynamo worker unavailable** — kill one vLLM worker; Dynamo removes it from
  rotation; measure error spike and recovery. Use `tests/failure/kill_dynamo_worker.sh`.
- **Malformed backend response** — set `MOCK_MALFORMED=1` on mock Triton; the
  gateway returns `upstream_malformed` (502), does not crash.
- **Client cancellation** — disconnect mid-stream; the handler future is dropped,
  releasing the admission permit and cancelling the upstream `reqwest` call.

## Helper scripts

```
tests/failure/kill_triton.sh          # simulate Triton outage
tests/failure/kill_dynamo_worker.sh   # simulate LLM worker loss
tests/failure/slow_backend.py         # restart a mock with injected latency
tests/failure/saturate_queue.py       # concurrency storm / load-shedding check
tests/failure/run_failure_suite.sh    # orchestrates all scenarios, PASS/FAIL
tests/failure/failure_config.yaml     # tiny limits to make overload easy to hit
```
