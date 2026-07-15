#!/usr/bin/env python3
"""Measure one request path's latency and throughput distribution.

Sends a fixed number of requests at a chosen concurrency and reports throughput
and latency percentiles. Compare `--target` pointed at the gateway vs. pointed
directly at a backend. Independent percentile distributions must be reported
side by side; subtracting their percentiles does not produce an overhead
distribution. Use paired per-request observations for that claim.

Against the mock backends this is a smoke test, NOT a publishable benchmark —
real numbers require the GPU stack and a recorded environment (see
docs/benchmarking.md). Output is JSON to stdout and optionally a file.
"""

import argparse
import json
import statistics
import sys
import time
import uuid
from concurrent.futures import ThreadPoolExecutor
from urllib import error, request


def one_request(target: str, path: str, body: dict) -> float:
    data = json.dumps(body).encode()
    req = request.Request(
        f"{target}{path}",
        data=data,
        headers={"Content-Type": "application/json", "x-request-id": str(uuid.uuid4())},
        method="POST",
    )
    start = time.perf_counter()
    try:
        resp = request.urlopen(req, timeout=30)
        resp.read()
        ok = 200 <= resp.status < 300
    except error.HTTPError as e:
        e.read()
        ok = False
    except Exception:
        ok = False
    elapsed_ms = (time.perf_counter() - start) * 1000.0
    return elapsed_ms if ok else -elapsed_ms


def percentile(values, p):
    if not values:
        return 0.0
    values = sorted(values)
    k = int(round((p / 100.0) * (len(values) - 1)))
    return values[k]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--target", default="http://localhost:8080")
    ap.add_argument("--path", default="/v1/infer/resnet50")
    ap.add_argument("--requests", type=int, default=500)
    ap.add_argument("--concurrency", type=int, default=8)
    ap.add_argument("--warmup", type=int, default=25)
    ap.add_argument("--body", default='{"inputs":[0.1,0.2,0.3]}')
    ap.add_argument("--out", default=None)
    args = ap.parse_args()

    body = json.loads(args.body)
    if args.requests <= 0 or args.concurrency <= 0 or args.warmup < 0:
        ap.error("requests and concurrency must be positive; warmup cannot be negative")

    # Warm-up is intentionally excluded from measured samples so connection
    # establishment and lazy initialization do not distort steady-state data.
    for _ in range(args.warmup):
        one_request(args.target, args.path, body)
    latencies = []
    errors = 0

    wall_start = time.perf_counter()
    with ThreadPoolExecutor(max_workers=args.concurrency) as pool:
        futures = [
            pool.submit(one_request, args.target, args.path, body)
            for _ in range(args.requests)
        ]
        for f in futures:
            v = f.result()
            if v < 0:
                errors += 1
            else:
                latencies.append(v)
    wall = time.perf_counter() - wall_start

    result = {
        "target": args.target,
        "path": args.path,
        "requests": args.requests,
        "warmup_requests": args.warmup,
        "concurrency": args.concurrency,
        "errors": errors,
        "successes": len(latencies),
        "throughput_rps": round(len(latencies) / wall, 2) if wall > 0 else 0.0,
        "latency_ms": {
            "p50": round(percentile(latencies, 50), 3),
            "p90": round(percentile(latencies, 90), 3),
            "p95": round(percentile(latencies, 95), 3),
            "p99": round(percentile(latencies, 99), 3),
            "max": round(max(latencies), 3) if latencies else 0.0,
            "mean": round(statistics.fmean(latencies), 3) if latencies else 0.0,
        },
        "note": "MOCK smoke test unless run against the GPU stack with a recorded environment.",
    }
    text = json.dumps(result, indent=2)
    print(text)
    if args.out:
        with open(args.out, "w") as f:
            f.write(text)
    # Non-zero exit if everything failed, so CI can catch a dead gateway.
    return 1 if errors == args.requests else 0


if __name__ == "__main__":
    sys.exit(main())
