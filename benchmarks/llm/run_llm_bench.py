#!/usr/bin/env python3
"""LLM benchmark driver (throughput, TTFT, inter-token latency).

Drives `/v1/chat/completions` at a chosen concurrency, parsing streamed SSE to
measure time-to-first-token and inter-token latency. Against mock Dynamo this
validates the harness; real numbers require the GPU stack.

For rigorous LLM benchmarking on real hardware, prefer NVIDIA GenAI-Perf /
Dynamo's documented benchmarking flow (see docs/benchmarking.md). This script is
the gateway-side companion that measures what clients actually observe.
"""
import argparse
import json
import statistics
import sys
import time
import uuid
from concurrent.futures import ThreadPoolExecutor
from urllib import request


def stream_once(target: str, model: str, prompt: str):
    body = json.dumps(
        {"model": model, "stream": True, "messages": [{"role": "user", "content": prompt}]}
    ).encode()
    req = request.Request(
        f"{target}/v1/chat/completions",
        data=body,
        headers={"Content-Type": "application/json", "x-request-id": str(uuid.uuid4())},
        method="POST",
    )
    start = time.perf_counter()
    ttft = None
    token_times = []
    try:
        resp = request.urlopen(req, timeout=60)
        for line in resp:
            line = line.decode().strip()
            if not line.startswith("data:"):
                continue
            payload = line[5:].strip()
            if payload == "[DONE]":
                break
            now = time.perf_counter()
            if ttft is None:
                ttft = now - start
            token_times.append(now)
    except Exception:
        return None
    end = time.perf_counter()
    inter = [t2 - t1 for t1, t2 in zip(token_times, token_times[1:])]
    return {
        "ttft_ms": (ttft or 0) * 1000.0,
        "e2e_ms": (end - start) * 1000.0,
        "tokens": len(token_times),
        "itl_ms": (statistics.fmean(inter) * 1000.0) if inter else 0.0,
    }


def pct(xs, p):
    if not xs:
        return 0.0
    xs = sorted(xs)
    return xs[int(round((p / 100.0) * (len(xs) - 1)))]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--target", default="http://localhost:8080")
    ap.add_argument("--model", default="qwen_small")
    ap.add_argument("--requests", type=int, default=64)
    ap.add_argument("--concurrency", type=int, default=8)
    ap.add_argument("--warmup", type=int, default=4)
    ap.add_argument("--prompt", default="Explain circuit breakers in one sentence.")
    ap.add_argument("--out", default=None)
    args = ap.parse_args()
    if args.requests <= 0 or args.concurrency <= 0 or args.warmup < 0:
        ap.error("requests and concurrency must be positive; warmup cannot be negative")

    for _ in range(args.warmup):
        stream_once(args.target, args.model, args.prompt)

    results = []
    wall_start = time.perf_counter()
    with ThreadPoolExecutor(max_workers=args.concurrency) as pool:
        futs = [
            pool.submit(stream_once, args.target, args.model, args.prompt)
            for _ in range(args.requests)
        ]
        for f in futs:
            r = f.result()
            if r:
                results.append(r)
    wall = time.perf_counter() - wall_start
    errors = args.requests - len(results)

    ttfts = [r["ttft_ms"] for r in results]
    e2es = [r["e2e_ms"] for r in results]
    total_tokens = sum(r["tokens"] for r in results)

    out = {
        "target": args.target,
        "model": args.model,
        "requests": args.requests,
        "warmup_requests": args.warmup,
        "concurrency": args.concurrency,
        "errors": errors,
        "throughput_rps": round(len(results) / wall, 2) if wall > 0 else 0.0,
        "sse_events_per_sec": round(total_tokens / wall, 2) if wall > 0 else 0.0,
        "successes": len(results),
        "ttft_ms": {
            "p50": round(pct(ttfts, 50), 2),
            "p90": round(pct(ttfts, 90), 2),
            "p95": round(pct(ttfts, 95), 2),
            "p99": round(pct(ttfts, 99), 2),
            "max": round(max(ttfts), 2) if ttfts else 0.0,
        },
        "e2e_ms": {
            "p50": round(pct(e2es, 50), 2),
            "p90": round(pct(e2es, 90), 2),
            "p95": round(pct(e2es, 95), 2),
            "p99": round(pct(e2es, 99), 2),
            "max": round(max(e2es), 2) if e2es else 0.0,
        },
        "note": "MOCK harness validation unless run against the GPU stack.",
    }
    text = json.dumps(out, indent=2)
    print(text)
    if args.out:
        with open(args.out, "w") as f:
            f.write(text)
    return 1 if errors == args.requests else 0


if __name__ == "__main__":
    sys.exit(main())
