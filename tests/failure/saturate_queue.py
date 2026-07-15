#!/usr/bin/env python3
"""Fire many concurrent requests at a slow backend and assert graceful shedding.

With max_concurrency=1 and queue_capacity=2 (failure_config.yaml) and a slow
Triton mock, most concurrent requests must be rejected with 429/503 rather than
piling up unbounded. The gateway must stay responsive throughout.

Exit 0 if: some requests were shed (429/503) AND the gateway answered /healthz
after the storm. Exit 1 otherwise.
"""
import sys
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from urllib import error

BASE = sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:18780"
N = int(sys.argv[2]) if len(sys.argv) > 2 else 40


def hit(_i):
    req = urllib.request.Request(
        f"{BASE}/v1/infer/resnet50",
        data=b'{"inputs":[1,2,3]}',
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        r = urllib.request.urlopen(req, timeout=10)
        return r.status
    except error.HTTPError as e:
        return e.code
    except Exception:
        return 0


def main():
    with ThreadPoolExecutor(max_workers=N) as pool:
        codes = list(pool.map(hit, range(N)))
    from collections import Counter

    counts = Counter(codes)
    print(f"status distribution: {dict(counts)}")

    shed = counts.get(429, 0) + counts.get(503, 0)
    # Gateway must still be alive.
    try:
        alive = urllib.request.urlopen(f"{BASE}/healthz", timeout=5).status == 200
    except Exception:
        alive = False
    print(f"shed={shed} gateway_alive={alive}")
    if shed > 0 and alive:
        print("PASS: overload shed gracefully, gateway stayed up")
        return 0
    print("FAIL: expected some 429/503 and a live gateway")
    return 1


if __name__ == "__main__":
    sys.exit(main())
