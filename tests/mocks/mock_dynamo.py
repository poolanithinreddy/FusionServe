#!/usr/bin/env python3
"""Mock NVIDIA Dynamo frontend (OpenAI-compatible subset) for GPU-free testing.

  GET  /health                    -> 200 when healthy
  POST /v1/chat/completions       -> OpenAI-style completion, streaming or not

Streaming responses emit Server-Sent-Events chunks (`data: {...}\\n\\n`) ending
with `data: [DONE]`, matching the vLLM/OpenAI streaming format the gateway
proxies through.

Failure-injection knobs (env vars):
  MOCK_LATENCY_MS   per-request latency (default 0)
  MOCK_TTFT_MS      delay before the first streamed token (default 20)
  MOCK_FAIL_RATE    fraction of calls returning 503
  MOCK_UNHEALTHY    if "1", /health returns 503

Pure standard library.
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


TOKENS = ["Hello", "!", " I", " am", " a", " mock", " Dynamo", " worker", "."]


class Handler(BaseHTTPRequestHandler):
    server_version = "MockDynamo/0.1"

    def log_message(self, fmt, *args):
        pass

    def _json(self, code, obj):
        body = json.dumps(obj).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if self.path == "/health":
            if os.environ.get("MOCK_UNHEALTHY") == "1":
                self._json(503, {"status": "unhealthy"})
            else:
                self._json(200, {"status": "ok"})
            return
        self._json(404, {"error": "not found"})

    def do_POST(self):
        if self.path != "/v1/chat/completions":
            self._json(404, {"error": "not found"})
            return

        length = int(self.headers.get("Content-Length", 0))
        raw = self.rfile.read(length)
        try:
            req = json.loads(raw or b"{}")
        except json.JSONDecodeError:
            self._json(400, {"error": "invalid json"})
            return

        latency = env_float("MOCK_LATENCY_MS", 0.0) / 1000.0
        if latency > 0:
            time.sleep(latency)

        fail_rate = env_float("MOCK_FAIL_RATE", 0.0)
        if fail_rate > 0 and random.random() < fail_rate:
            self._json(503, {"error": "no workers available"})
            return

        model = req.get("model", "qwen_small")
        if req.get("stream"):
            self._stream(model)
        else:
            self._unary(model, self.headers.get("x-request-id"))

    def _unary(self, model, request_id):
        text = "".join(TOKENS)
        self._json(
            200,
            {
                "id": "chatcmpl-mock",
                "object": "chat.completion",
                "model": model,
                "received_request_id": request_id,
                "choices": [
                    {
                        "index": 0,
                        "message": {"role": "assistant", "content": text},
                        "finish_reason": "stop",
                    }
                ],
                "usage": {
                    "prompt_tokens": 8,
                    "completion_tokens": len(TOKENS),
                    "total_tokens": 8 + len(TOKENS),
                },
            },
        )

    def _stream(self, model):
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-cache")
        self.end_headers()
        ttft = env_float("MOCK_TTFT_MS", 20.0) / 1000.0
        if ttft > 0:
            time.sleep(ttft)
        for i, tok in enumerate(TOKENS):
            chunk = {
                "id": "chatcmpl-mock",
                "object": "chat.completion.chunk",
                "model": model,
                "choices": [
                    {"index": 0, "delta": {"content": tok}, "finish_reason": None}
                ],
            }
            self.wfile.write(f"data: {json.dumps(chunk)}\n\n".encode())
            self.wfile.flush()
            if i < len(TOKENS) - 1:
                time.sleep(0.005)  # inter-token delay
        self.wfile.write(b"data: [DONE]\n\n")
        self.wfile.flush()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=8000)
    ap.add_argument("--host", default="0.0.0.0")
    args = ap.parse_args()
    srv = ThreadingHTTPServer((args.host, args.port), Handler)
    srv.daemon_threads = True
    print(f"mock-dynamo listening on {args.host}:{args.port}", flush=True)
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        srv.shutdown()


if __name__ == "__main__":
    main()
