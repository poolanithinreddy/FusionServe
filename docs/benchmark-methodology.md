# Benchmark methodology

## Result classes

Every run declares a backend type (`mock`, `triton`, `dynamo`, or `vllm`) and a
hardware type (`cpu`, `single_gpu`, or `multi_gpu`). Mock results validate the
gateway and harness; they are not proxies for model-serving performance.

## Experimental controls

A publishable run records the Git commit, benchmark version, sanitized hardware,
software versions, model and revision, input distribution, warm-up, concurrency,
request count, repetition count, and units. Important GPU configurations require
at least three measured repetitions. Raw runs are retained without selecting
only the fastest repetition.

Latency-oriented and throughput-oriented experiments are separate. Within a
comparison, inputs, model configuration, protocol, batching, instance count,
warm-up, and measurement window remain identical. Report failures alongside
successes and state whether throughput counts requests or model tokens.

## Latency and gateway overhead

Each distribution reports `p50`, `p90`, `p95`, `p99`, and `max`, all from the
same sample. Percentiles must be monotonic. Direct-backend and gateway results
from independent runs are reported side by side.

Subtracting independent percentiles is prohibited. A gateway overhead
distribution is valid only when controlled request pairs are retained and each
sample is calculated as `gateway_latency_i - direct_latency_i` before computing
percentiles. Without paired samples, FusionServe reports no tail-overhead
percentiles.

## Integrity gate

Run:

```bash
python3 benchmarks/validate_results.py artifacts/benchmarks/mock
```

The validator rejects missing benchmark/hardware metadata, duplicate run IDs,
invalid counts or error rates, negative/non-finite measurements, inconsistent
GPU classification, and non-monotonic percentiles. Legacy summaries that lack
`p90` or `max` retain only the percentiles actually measured; new runs must
include the complete distribution.
