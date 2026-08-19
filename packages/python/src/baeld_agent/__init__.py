"""Framework-neutral Baeld phase client and Browser Use adapter."""

from __future__ import annotations

import asyncio
import json
import os
from dataclasses import dataclass
from typing import Any, Awaitable, Callable, TypeVar

T = TypeVar("T")


@dataclass(frozen=True)
class Session:
    id: str
    cdp_url: str
    filter_profile: str
    mode: str


class BaeldAgent:
    def __init__(
        self,
        reader: asyncio.StreamReader,
        writer: asyncio.StreamWriter,
        session_id: str,
        framework: str | None = None,
        workload: str | None = None,
    ) -> None:
        self._reader = reader
        self._writer = writer
        self._session_id = session_id
        self._framework = framework
        self._workload = workload
        self._generation = 0
        self.previous_phase: str | None = None

    @classmethod
    async def connect(
        cls,
        *,
        socket_path: str | None = None,
        session_id: str | None = None,
        framework: str | None = None,
        workload: str | None = None,
    ) -> "BaeldAgent":
        socket_path = socket_path or os.environ.get("BAELD_PHASE_SOCKET")
        session_id = session_id or os.environ.get("BAELD_SESSION_ID")
        if not socket_path or not session_id:
            raise RuntimeError("Run through `baeld run` or provide socket_path and session_id")
        reader, writer = await asyncio.open_unix_connection(socket_path)
        return cls(reader, writer, session_id, framework, workload)

    async def phase(
        self,
        name: str,
        *,
        expected_wait_ms: int | None = None,
        critical_live_connection: bool = False,
        timeout: float = 15.0,
    ) -> dict[str, Any]:
        self._generation += 1
        request = {
            "schema_version": 1,
            "session_id": self._session_id,
            "generation": self._generation,
            "phase": name,
            "expected_wait_ms": expected_wait_ms,
            "framework": self._framework,
            "workload": self._workload,
            "critical_live_connection": critical_live_connection,
        }
        self._writer.write((json.dumps(request, separators=(",", ":")) + "\n").encode())
        await self._writer.drain()
        raw = await asyncio.wait_for(self._reader.readline(), timeout)
        if not raw:
            raise ConnectionError("Baeld phase socket closed")
        ack = json.loads(raw)
        if ack.get("generation") != self._generation:
            raise RuntimeError("Baeld acknowledgement generation mismatch")
        if not ack.get("accepted"):
            raise RuntimeError(ack.get("error") or "Phase rejected")
        self.previous_phase = name
        return ack

    async def waiting_for_model(
        self, expected_wait_ms: int, *, critical_live_connection: bool = False
    ) -> dict[str, Any]:
        return await self.phase(
            "waiting_for_model",
            expected_wait_ms=expected_wait_ms,
            critical_live_connection=critical_live_connection,
        )

    async def acting(self) -> dict[str, Any]:
        return await self.phase("acting")

    async def verify(self) -> dict[str, Any]:
        return await self.phase("verifying")

    async def finish(self) -> dict[str, Any]:
        return await self.phase("finished")

    def session(self) -> Session:
        return Session(
            self._session_id,
            os.environ.get("BAELD_CDP_URL", ""),
            os.environ.get("BAELD_FILTER_PROFILE", "safe"),
            os.environ.get("BAELD_SESSION_MODE", "ephemeral"),
        )

    async def close(self) -> None:
        self._writer.close()
        await self._writer.wait_closed()


class BaeldPhaseModel:
    """Wrap a Browser Use BaseChatModel and expose each inference boundary."""

    def __init__(
        self,
        model: Any,
        agent: BaeldAgent,
        *,
        expected_wait_ms: int = 5_000,
        critical_live_connection: bool = False,
    ) -> None:
        self._model = model
        self._agent = agent
        self.expected_wait_ms = expected_wait_ms
        self.critical_live_connection = critical_live_connection
        self.model = model.model
        self._verified_api_keys = getattr(model, "_verified_api_keys", False)

    @property
    def provider(self) -> str:
        return self._model.provider

    @property
    def name(self) -> str:
        return self._model.name

    @property
    def model_name(self) -> str:
        return self._model.model_name

    async def ainvoke(
        self, messages: list[Any], output_format: Any = None, **kwargs: Any
    ) -> Any:
        if self._agent.previous_phase in {"acting", "verifying"}:
            await self._agent.phase("observing")
        await self._agent.waiting_for_model(
            self.expected_wait_ms,
            critical_live_connection=self.critical_live_connection,
        )
        try:
            return await self._model.ainvoke(messages, output_format, **kwargs)
        finally:
            await self._agent.acting()


async def connect_browser_use(**options: Any) -> tuple[Any, BaeldAgent]:
    from browser_use import BrowserSession

    cdp_url = options.pop("cdp_url", None) or os.environ.get("BAELD_CDP_URL")
    if not cdp_url:
        raise RuntimeError("BAELD_CDP_URL is not set")
    browser = BrowserSession(cdp_url=cdp_url, is_local=True, keep_alive=True, **options)
    await browser.start()
    return browser, await BaeldAgent.connect(framework="browser-use")


async def with_model_wait(
    agent: BaeldAgent,
    expected_wait_ms: int,
    operation: Callable[[], Awaitable[T]],
    *,
    critical_live_connection: bool = False,
) -> T:
    await agent.waiting_for_model(
        expected_wait_ms, critical_live_connection=critical_live_connection
    )
    try:
        return await operation()
    finally:
        await agent.acting()


__all__ = [
    "BaeldAgent",
    "BaeldPhaseModel",
    "Session",
    "connect_browser_use",
    "with_model_wait",
]
