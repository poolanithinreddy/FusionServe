"""End-to-end integration test: real gateway binary + mock backends.

Builds (if needed) and launches the gateway against the mock Triton and Dynamo
servers, then drives it through the Python client. This is the test that proves
the full request path — routing, forwarding, streaming, error mapping — works,
without needing a GPU.

Run:  pytest tests/integration/test_end_to_end.py -q
Skips automatically if cargo is unavailable.
"""
import json
import os
import shutil
import subprocess
import sys
import time
import urllib.request
import urllib.error
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "clients" / "python"))

from fusionserve import ChatMessage, ChatRequest, FusionServeClient  # noqa: E402
from fusionserve.client import FusionServeError  # noqa: E402

TRITON_PORT = 18601
DYNAMO_PORT = 18600
GATEWAY_PORT = 18680


def _wait_http(url: str, timeout: float = 20.0) -> bool:
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            with urllib.request.urlopen(url, timeout=1) as r:
                if r.status == 200:
                    return True
        except Exception:
            time.sleep(0.2)
    return False


@pytest.fixture(scope="module")
def stack():
    if shutil.which("cargo") is None:
        pytest.skip("cargo not available")

    # Build the gateway (fast if already built).
    subprocess.run(
        ["cargo", "build", "--quiet", "--offline", "-p", "fusionserve-gateway"],
        cwd=ROOT,
        check=True,
    )
    binary = ROOT / "target" / "debug" / "fusionserve-gateway"

    # Write a config pointing at our test ports.
    cfg = ROOT / "tests" / "integration" / "_e2e_config.yaml"
    cfg.write_text(
        f"""
server: {{ bind_addr: "127.0.0.1:{GATEWAY_PORT}", global_max_inflight: 128, default_deadline_ms: 5000 }}
admission:
  queue_capacity: 64
  queue_timeout_ms: 1000
  circuit_breaker: {{ failure_threshold: 3, open_cooldown_ms: 500, half_open_success_threshold: 1 }}
health: {{ poll_interval_ms: 1000, unhealthy_after: 3 }}
models:
  resnet50:
    workload: image_classification
    backend: triton
    protocol: http
    endpoint: "http://127.0.0.1:{TRITON_PORT}"
    timeout_ms: 2000
    max_concurrency: 16
  qwen_small:
    workload: chat_completion
    backend: dynamo
    protocol: http
    endpoint: "http://127.0.0.1:{DYNAMO_PORT}"
    timeout_ms: 5000
    max_concurrency: 8
    streaming: true
"""
    )

    procs = []
    procs.append(
        subprocess.Popen(
            [sys.executable, str(ROOT / "tests/mocks/mock_triton.py"), "--port", str(TRITON_PORT)]
        )
    )
    procs.append(
        subprocess.Popen(
            [sys.executable, str(ROOT / "tests/mocks/mock_dynamo.py"), "--port", str(DYNAMO_PORT)]
        )
    )
    env = dict(os.environ, FUSIONSERVE_CONFIG=str(cfg), RUST_LOG="warn")
    procs.append(subprocess.Popen([str(binary)], env=env))

    try:
        assert _wait_http(f"http://127.0.0.1:{GATEWAY_PORT}/readyz"), "gateway did not become ready"
        yield f"http://127.0.0.1:{GATEWAY_PORT}"
    finally:
        for p in procs:
            p.terminate()
        for p in procs:
            try:
                p.wait(timeout=5)
            except subprocess.TimeoutExpired:
                p.kill()
        cfg.unlink(missing_ok=True)


def test_health_and_models(stack):
    c = FusionServeClient(stack)
    assert c.health()
    models = c.list_models()
    names = {m["name"] for m in models["data"]}
    assert {"resnet50", "qwen_small"} <= names


def test_infer_routes_to_triton(stack):
    c = FusionServeClient(stack)
    res = c.infer("resnet50", [{"name": "x", "datatype": "FP32", "shape": [1, 3], "data": [0.1, 0.2, 0.3]}], request_id="e2e-triton-1")
    assert res.raw["model_name"] == "resnet50"
    assert res.request_id == "e2e-triton-1"
    assert res.raw["received_request_id"] == "e2e-triton-1"


def test_chat_unary(stack):
    c = FusionServeClient(stack)
    res = c.chat(
        ChatRequest(model="qwen_small", messages=[ChatMessage("user", "hi")]),
        request_id="e2e-dynamo-1",
    )
    assert "mock Dynamo" in res.content
    assert res.raw["received_request_id"] == "e2e-dynamo-1"


def test_chat_streaming(stack):
    c = FusionServeClient(stack)
    chunks = list(
        c.stream_chat(ChatRequest(model="qwen_small", messages=[ChatMessage("user", "hi")]))
    )
    assert "".join(chunks).strip().startswith("Hello")


def test_unknown_model_error_contract(stack):
    c = FusionServeClient(stack)
    with pytest.raises(FusionServeError) as ei:
        c.infer("nope", [])
    assert ei.value.code == "unknown_model"
    assert ei.value.status == 404


def test_chat_model_rejected_on_infer(stack):
    c = FusionServeClient(stack)
    with pytest.raises(FusionServeError) as ei:
        c.infer("qwen_small", [])
    assert ei.value.code == "bad_request"


def test_inflight_metrics_return_to_zero(stack):
    with urllib.request.urlopen(f"{stack}/metrics", timeout=2) as response:
        metrics = response.read().decode()
    assert 'fusionserve_inflight_requests{backend="triton",model="resnet50"} 0' in metrics
    assert 'fusionserve_inflight_requests{backend="dynamo",model="qwen_small"} 0' in metrics


def test_downstream_deadline_returns_504(stack):
    body = b'{"model":"resnet50","inputs":[1],"mock_delay_ms":200}'
    req = urllib.request.Request(
        f"{stack}/v1/infer",
        data=body,
        headers={"content-type": "application/json", "x-deadline-ms": "25"},
        method="POST",
    )
    with pytest.raises(urllib.error.HTTPError) as exc:
        urllib.request.urlopen(req, timeout=2)
    assert exc.value.code == 504
    payload = json.loads(exc.value.read())
    assert payload["error"]["code"] == "deadline_exceeded"


def test_stream_disconnect_increments_cancellation_metric(stack):
    body = b'{"model":"qwen_small","stream":true,"messages":[{"role":"user","content":"hi"}]}'
    req = urllib.request.Request(
        f"{stack}/v1/chat/completions",
        data=body,
        headers={"content-type": "application/json"},
        method="POST",
    )
    response = urllib.request.urlopen(req, timeout=2)
    assert response.readline().startswith(b"data:")
    response.close()
    time.sleep(0.1)
    with urllib.request.urlopen(f"{stack}/metrics", timeout=2) as metrics_response:
        metrics = metrics_response.read().decode()
    assert "fusionserve_request_cancellations_total" in metrics
    assert 'backend="dynamo",model="qwen_small",workload="chat_completion"} 1' in metrics
