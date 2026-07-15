# CPU/mock fault-injection validation

Environment: Apple M4, macOS arm64, no NVIDIA GPU. These experiments exercise
the Rust gateway with deterministic mock Triton and Dynamo services.

Commands:

```bash
bash tests/failure/run_failure_suite.sh
python3 -m pytest tests/integration/test_end_to_end.py -q
```

| Experiment | Expected | Actual evidence | Result |
|---|---|---|---|
| Stop Triton during traffic | Non-LLM fails quickly; LLM works | Six inference requests returned 503; chat returned 200 | PASS |
| Restart Triton | Non-LLM recovers | Subsequent slow-backend inference returned 200 | PASS |
| Stop Dynamo worker | LLM fails quickly; Triton works | Chat returned 503; inference returned 200 | PASS |
| Inject backend latency | Latency remains bounded | 800 ms injected request completed in 826 ms | PASS |
| Trigger deadline | Gateway returns 504 | 25 ms deadline around 200 ms mock delay returned `deadline_exceeded` | PASS |
| Malformed response | Gateway returns safe 502 | Invalid mock JSON mapped to `upstream_malformed` | PASS |
| Saturate admission | Reject excess load; stay alive | 39/40 requests shed; gateway health remained 200 | PASS |
| Circuit recovery | Closed → open → half-open → closed | Three forced 503s opened circuit; post-cooldown probe returned 200 | PASS |
| Disconnect streaming client | Release permit and count cancellation | Stream closed after first event; cancellation counter increased | PASS |
| Unknown model | Stable client error | Request returned 404 with `unknown_model` | PASS |

Detection and recovery timings are mock-specific and must not be presented as
NVIDIA Triton or Dynamo production measurements.
