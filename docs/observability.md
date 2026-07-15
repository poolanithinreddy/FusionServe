# Observability

FusionServe exports Prometheus metrics at `/metrics`, structured JSON logs, and
request correlation through `x-request-id`. Metric labels are restricted to
configured model/backend/workload identifiers and stable outcome codes. Full
backend URLs, request IDs, prompts, user input, session IDs, and arbitrary error
text are not metric labels.

The provisioned Grafana dashboard covers request rate, p50/p95/p99 latency,
in-flight work, queue depth, circuit state, backend health, LLM time to first
token, retries, errors, cancellations, and—when DCGM Exporter is running—GPU
utilization and framebuffer memory.

```bash
docker compose --profile mock up --build -d
open http://localhost:3000
```

The GPU profile adds the pinned NVIDIA DCGM Exporter. GPU panels remain empty in
CPU/mock mode; no screenshot is included because this machine has no NVIDIA GPU
and fake dashboard data would be misleading.
