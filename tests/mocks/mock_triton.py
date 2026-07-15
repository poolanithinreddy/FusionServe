#!/usr/bin/env python3
"""Mock NVIDIA Triton server (KServe v2 subset) for GPU-free testing.

Implements just enough of Triton's HTTP surface for the gateway to route to it:

  GET  /v2/health/ready          -> 200 when healthy
  GET  /v2/health/live           -> 200
  GET  /v2/models/{model}        -> model metadata
  POST /v2/models/{model}/infer  -> deterministic fake inference response

Failure-injection knobs (env vars, read per request so they can change live):
  MOCK_LATENCY_MS   add this much latency to /infer   (default 0)
  MOCK_FAIL_RATE    fraction of /infer calls returning 503  (0.0-1.0)
  MOCK_MALFORMED    if "1", return invalid JSON from /infer
  MOCK_UNHEALTHY    if "1", /v2/health/ready returns 503

Pure standard library: no external dependencies.
"""
import argparse
import json
import os
import random
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


def env_float(name, default):
    try:
        return float(os.environ.get(name, default))
    except ValueError:
        return default


class Handler(BaseHTTPRequestHandler):
    server_version = "MockTriton/0.1"

    def log_message(self, fmt, *args):  # quieter logs
        pass

    def _json(self, code, obj):
        body = obj if isinstance(obj, bytes) else json.dumps(obj).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if self.path in ("/v2/health/ready",):
            if os.environ.get("MOCK_UNHEALTHY") == "1":
                self._json(503, {"error": "unhealthy"})
            else:
                self._json(200, {"ready": True})
            return
        if self.path == "/v2/health/live":
            self._json(200, {"live": True})
            return
        if self.path.startswith("/v2/models/"):
            model = self.path.split("/v2/models/")[1].split("/")[0]
            self._json(200, {"name": model, "platform": "mock", "versions": ["1"]})
            return
        self._json(404, {"error": "not found"})

    def do_POST(self):
        if "/infer" not in self.path:
            self._json(404, {"error": "not found"})
            return

        length = int(self.headers.get("Content-Length", 0))
        _ = self.rfile.read(length)  # consume body; content is not needed for the mock
        model = self.path.split("/v2/models/")[1].split("/")[0]

        latency = env_float("MOCK_LATENCY_MS", 0.0) / 1000.0
        if latency > 0:
            time.sleep(latency)

        fail_rate = env_float("MOCK_FAIL_RATE", 0.0)
        if fail_rate > 0 and random.random() < fail_rate:
            self._json(503, {"error": "mock backend overloaded"})
            return

        if os.environ.get("MOCK_MALFORMED") == "1":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(b'{"this is": not valid json')
            return

        # Deterministic fake KServe v2 inference response.
        response = {
            "model_name": model,
            "model_version": "1",
            "outputs": [
                {
                    "name": "top5",
                    "datatype": "FP32",
                    "shape": [1, 5],
                    "data": [0.71, 0.12, 0.06, 0.03, 0.01],
                },
                {
                    "name": "labels",
                    "datatype": "BYTES",
                    "shape": [1, 5],
                    "data": ["tabby", "tiger_cat", "Egyptian_cat", "lynx", "cougar"],
                },
            ],
        }
        self._json(200, response)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=8001)
    ap.add_argument("--host", default="0.0.0.0")
    args = ap.parse_args()
    srv = ThreadingHTTPServer((args.host, args.port), Handler)
    print(f"mock-triton listening on {args.host}:{args.port}", flush=True)
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        srv.shutdown()


if __name__ == "__main__":
    main()
