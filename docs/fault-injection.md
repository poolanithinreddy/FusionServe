# Fault injection

Run the complete CPU/mock suite:

```bash
scripts/fault-injection/run_cpu_mock.sh
```

It covers Triton loss/recovery, Dynamo loss, injected latency, deadlines,
malformed responses, admission saturation, circuit open/half-open recovery,
stream cancellation, and unknown models. Expected behavior and recorded results
are in [`artifacts/fault-injection/cpu-mock-validation.md`](../artifacts/fault-injection/cpu-mock-validation.md).

These deterministic experiments validate gateway policy. Real component
detection and recovery times must be measured again on the NVIDIA deployment.
