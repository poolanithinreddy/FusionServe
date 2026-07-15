"""Async FusionServe client for load generation.

Uses ``asyncio`` + the stdlib client under a thread executor so it stays
dependency-free while still driving high concurrency. For production use you
would swap in ``httpx``/``aiohttp``; the interface is intentionally identical to
make that swap trivial.
"""
from __future__ import annotations

import asyncio
from typing import Any, List, Optional

from .client import FusionServeClient
from .types import ChatRequest, ChatResult, InferResult


class AsyncClient:
    def __init__(self, base_url: str = "http://localhost:8080", timeout: float = 35.0):
        self._sync = FusionServeClient(base_url, timeout)

    async def infer(self, model: str, inputs: Any, request_id: Optional[str] = None) -> InferResult:
        return await asyncio.to_thread(self._sync.infer, model, inputs, request_id)

    async def chat(self, req: ChatRequest, request_id: Optional[str] = None) -> ChatResult:
        return await asyncio.to_thread(self._sync.chat, req, request_id)

    async def gather_infer(self, model: str, inputs_list: List[Any]) -> List[InferResult]:
        return await asyncio.gather(*(self.infer(model, i) for i in inputs_list))


__all__ = ["AsyncClient"]
