"""Synchronous FusionServe client (standard library only).

Intentionally dependency-free (uses ``urllib``) so it runs in any environment,
including CI without a package install. For high-throughput load generation see
``async_client.AsyncClient``.
"""
from __future__ import annotations

import json
import uuid
from typing import Any, Callable, Dict, Iterator, Optional
from urllib import error, request

from .types import ChatMessage, ChatRequest, ChatResult, FusionServeError, InferResult


class FusionServeClient:
    def __init__(self, base_url: str = "http://localhost:8080", timeout: float = 35.0):
        self.base_url = base_url.rstrip("/")
        self.timeout = timeout

    # -- internals ----------------------------------------------------------
    def _headers(self, request_id: Optional[str]) -> Dict[str, str]:
        return {
            "Content-Type": "application/json",
            "x-request-id": request_id or str(uuid.uuid4()),
        }

    def _post(self, path: str, body: Dict[str, Any], request_id: Optional[str]):
        data = json.dumps(body).encode()
        req = request.Request(
            f"{self.base_url}{path}", data=data, headers=self._headers(request_id), method="POST"
        )
        try:
            resp = request.urlopen(req, timeout=self.timeout)
            rid = resp.headers.get("x-request-id")
            return json.loads(resp.read().decode()), rid
        except error.HTTPError as e:
            self._raise_structured(e)

    def _raise_structured(self, e: "error.HTTPError"):
        try:
            payload = json.loads(e.read().decode())
            err = payload.get("error", {})
            raise FusionServeError(
                e.code,
                err.get("code", "unknown"),
                err.get("message", str(e)),
                err.get("request_id"),
            )
        except (ValueError, KeyError):
            raise FusionServeError(e.code, "unknown", str(e), None)

    # -- public API ---------------------------------------------------------
    def health(self) -> bool:
        try:
            resp = request.urlopen(f"{self.base_url}/healthz", timeout=self.timeout)
            return resp.status == 200
        except error.URLError:
            return False

    def list_models(self) -> Dict[str, Any]:
        resp = request.urlopen(f"{self.base_url}/v1/models", timeout=self.timeout)
        return json.loads(resp.read().decode())

    def infer(self, model: str, inputs: Any, request_id: Optional[str] = None) -> InferResult:
        raw, rid = self._post(f"/v1/infer/{model}", {"inputs": inputs}, request_id)
        return InferResult(model=model, raw=raw, request_id=rid)

    def embed(self, model: str, text: str, request_id: Optional[str] = None) -> InferResult:
        raw, rid = self._post("/v1/embeddings", {"model": model, "input": text}, request_id)
        return InferResult(model=model, raw=raw, request_id=rid)

    def chat(self, req: ChatRequest, request_id: Optional[str] = None) -> ChatResult:
        body = req.as_dict()
        body["stream"] = False
        raw, rid = self._post("/v1/chat/completions", body, request_id)
        content = (
            raw.get("choices", [{}])[0].get("message", {}).get("content", "")
        )
        return ChatResult(content=content, raw=raw, request_id=rid)

    def stream_chat(
        self,
        req: ChatRequest,
        on_chunk: Optional[Callable[[str], None]] = None,
        request_id: Optional[str] = None,
    ) -> Iterator[str]:
        """Yield content deltas from a streaming chat completion.

        Parses the SSE ``data: {...}`` frames the gateway proxies from Dynamo.
        """
        body = req.as_dict()
        body["stream"] = True
        data = json.dumps(body).encode()
        http_req = request.Request(
            f"{self.base_url}/v1/chat/completions",
            data=data,
            headers=self._headers(request_id),
            method="POST",
        )
        try:
            resp = request.urlopen(http_req, timeout=self.timeout)
        except error.HTTPError as e:
            self._raise_structured(e)
            return
        for line in resp:
            line = line.decode().strip()
            if not line.startswith("data:"):
                continue
            payload = line[len("data:"):].strip()
            if payload == "[DONE]":
                break
            try:
                obj = json.loads(payload)
            except ValueError:
                continue
            delta = obj.get("choices", [{}])[0].get("delta", {}).get("content")
            if delta:
                if on_chunk:
                    on_chunk(delta)
                yield delta


__all__ = [
    "FusionServeClient",
    "ChatRequest",
    "ChatMessage",
    "ChatResult",
    "InferResult",
    "FusionServeError",
]
