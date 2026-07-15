"""Typed request/response objects for the FusionServe Python client."""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional


class FusionServeError(Exception):
    """Raised when the gateway returns a structured error body.

    Mirrors the gateway's error contract so callers can branch on ``code``.
    """

    def __init__(self, status: int, code: str, message: str, request_id: Optional[str]):
        super().__init__(f"[{status} {code}] {message} (request_id={request_id})")
        self.status = status
        self.code = code
        self.message = message
        self.request_id = request_id


@dataclass
class InferResult:
    model: str
    raw: Dict[str, Any]
    request_id: Optional[str] = None


@dataclass
class ChatMessage:
    role: str
    content: str

    def as_dict(self) -> Dict[str, str]:
        return {"role": self.role, "content": self.content}


@dataclass
class ChatRequest:
    model: str
    messages: List[ChatMessage]
    max_tokens: Optional[int] = None
    temperature: Optional[float] = None
    stream: bool = False
    extra: Dict[str, Any] = field(default_factory=dict)

    def as_dict(self) -> Dict[str, Any]:
        body: Dict[str, Any] = {
            "model": self.model,
            "messages": [m.as_dict() for m in self.messages],
            "stream": self.stream,
        }
        if self.max_tokens is not None:
            body["max_tokens"] = self.max_tokens
        if self.temperature is not None:
            body["temperature"] = self.temperature
        body.update(self.extra)
        return body


@dataclass
class ChatResult:
    content: str
    raw: Dict[str, Any]
    request_id: Optional[str] = None
