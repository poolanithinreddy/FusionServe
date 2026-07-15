#!/usr/bin/env python3
"""Restart a mock backend with injected latency.

Usage: slow_backend.py [triton|dynamo] <port> <latency_ms>
Convenience wrapper; the mocks read MOCK_LATENCY_MS from the environment.
"""
import os, subprocess, sys
kind = sys.argv[1] if len(sys.argv) > 1 else "triton"
port = sys.argv[2] if len(sys.argv) > 2 else ("8001" if kind == "triton" else "8000")
latency = sys.argv[3] if len(sys.argv) > 3 else "500"
script = f"tests/mocks/mock_{kind}.py"
env = dict(os.environ, MOCK_LATENCY_MS=latency)
print(f"starting {script} on :{port} with {latency}ms latency")
subprocess.run([sys.executable, script, "--port", port], env=env)
