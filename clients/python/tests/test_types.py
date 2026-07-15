"""Unit tests for client types (no server required)."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from fusionserve.client import FusionServeClient  # noqa: E402
from fusionserve.types import ChatMessage, ChatRequest, FusionServeError  # noqa: E402


def test_chat_request_serialization_minimal():
    req = ChatRequest(model="qwen_small", messages=[ChatMessage("user", "hi")])
    body = req.as_dict()
    assert body["model"] == "qwen_small"
    assert body["messages"] == [{"role": "user", "content": "hi"}]
    assert body["stream"] is False
    assert "max_tokens" not in body


def test_chat_request_serialization_full():
    req = ChatRequest(
        model="qwen_small",
        messages=[ChatMessage("user", "hi")],
        max_tokens=64,
        temperature=0.7,
        stream=True,
        extra={"top_p": 0.9},
    )
    body = req.as_dict()
    assert body["max_tokens"] == 64
    assert body["temperature"] == 0.7
    assert body["stream"] is True
    assert body["top_p"] == 0.9


def test_error_str_includes_code_and_request_id():
    err = FusionServeError(503, "circuit_open", "breaker open", "rid-1")
    s = str(err)
    assert "circuit_open" in s
    assert "rid-1" in s


def test_infer_uses_unified_api_and_includes_model():
    client = FusionServeClient("http://example.invalid")
    captured = {}

    def fake_post(path, body, request_id):
        captured.update(path=path, body=body, request_id=request_id)
        return {"outputs": []}, "rid-2"

    client._post = fake_post
    result = client.infer("resnet50", [1, 2, 3], request_id="rid-2")
    assert captured == {
        "path": "/v1/infer",
        "body": {"model": "resnet50", "inputs": [1, 2, 3]},
        "request_id": "rid-2",
    }
    assert result.request_id == "rid-2"
