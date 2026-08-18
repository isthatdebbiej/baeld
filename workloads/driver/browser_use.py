#!/usr/bin/env python3
"""Deterministic Browser Use adapter for Baeld's ownership/correctness gate."""

from __future__ import annotations

import asyncio
import json
import os
import pathlib
import sys
import tempfile
import time
import traceback
import urllib.parse
import urllib.request
from typing import Any, Awaitable, Callable

os.environ.setdefault("ANONYMIZED_TELEMETRY", "false")
os.environ.setdefault("BROWSER_USE_LOGGING_LEVEL", "error")
os.environ.setdefault(
    "BROWSER_USE_CONFIG_DIR",
    str(pathlib.Path(tempfile.gettempdir()) / "baeld-browser-use-config"),
)

from browser_use import BrowserSession  # noqa: E402
from browser_use.logging_config import setup_logging  # noqa: E402

setup_logging(stream=sys.stderr, log_level="error", force_setup=True)


async def main() -> None:
    env = os.environ
    session_id = required(env, "BAELD_SESSION_ID")
    socket_path = required(env, "BAELD_SOCKET")
    cdp_url = required(env, "BAELD_CDP_URL")
    base_url = required(env, "BAELD_BASE_URL")
    workload = required(env, "BAELD_WORKLOAD")
    mechanism = required(env, "BAELD_MECHANISM")
    wait_ms = int(required(env, "BAELD_WAIT_MS"))
    settle_ms = int(required(env, "BAELD_SETTLE_MS"))
    if mechanism not in {"baseline", "cgroup-freeze-500ms"}:
        raise ValueError(f"Browser Use gate does not support mechanism {mechanism}")

    phase = await PhaseClient.connect(socket_path, session_id)
    started = time.perf_counter()
    resume_latency_ms = 0.0
    browser: BrowserSession | None = None
    result: dict[str, Any]

    try:
        await phase.set("starting")
        browser = BrowserSession(cdp_url=cdp_url, is_local=True, keep_alive=True)
        await browser.start()
        page = await browser.new_page()

        await http_json(f"{base_url}/api/reset?session={urllib.parse.quote(session_id)}", "POST")
        await phase.set("navigating")
        await page.goto(f"{base_url}/{workload}?session={urllib.parse.quote(session_id)}")
        await phase.set("observing")
        await wait_until(lambda: selector_exists(page, "h1"), 15.0)
        if workload == "websocket":
            await wait_until(lambda: websocket_ready(page), 15.0)

        background_before = await background_operations(page, workload, base_url, session_id)
        await phase.set("waiting_for_model", wait_ms)
        await asyncio.sleep(wait_ms / 1000)

        resume_started = time.perf_counter()
        await phase.set("acting")
        resume_latency_ms = (time.perf_counter() - resume_started) * 1000
        background_after = await background_operations(page, workload, base_url, session_id)

        if workload != "static":
            buttons = await page.get_elements_by_css_selector("#save")
            if len(buttons) != 1:
                raise RuntimeError(f"expected one #save button, found {len(buttons)}")
            await buttons[0].click()

        await phase.set("verifying")
        oracle = await verify(page, workload, base_url, session_id)
        await phase.set("settling")
        await asyncio.sleep(settle_ms / 1000)
        live = await live_state(page) if workload == "websocket" else {"reconnects": 0, "sequences": []}
        await phase.set("finished")

        gaps = sequence_gaps(live.get("sequences", []))
        result = {
            "success": oracle["success"] and gaps == 0,
            "latency_ms": (time.perf_counter() - started) * 1000,
            "resume_latency_ms": resume_latency_ms,
            "reconnects": live.get("reconnects", 0),
            "sequence_gaps": gaps,
            "background_operations": max(0, background_after - background_before),
            "failure": oracle.get("failure") or (f"{gaps} websocket sequence gaps" if gaps else None),
        }
    except Exception:
        result = {
            "success": False,
            "latency_ms": (time.perf_counter() - started) * 1000,
            "resume_latency_ms": resume_latency_ms,
            "reconnects": 0,
            "sequence_gaps": 0,
            "background_operations": 0,
            "failure": traceback.format_exc(),
        }
    finally:
        await phase.close()
        if browser is not None:
            try:
                await asyncio.wait_for(browser.stop(), timeout=3.0)
            except Exception:
                pass

    sys.stdout.write(json.dumps(result, separators=(",", ":")))
    sys.stdout.flush()


class PhaseClient:
    def __init__(self, reader: asyncio.StreamReader, writer: asyncio.StreamWriter, session_id: str):
        self.reader = reader
        self.writer = writer
        self.session_id = session_id
        self.generation = 0

    @classmethod
    async def connect(cls, path: str, session_id: str) -> "PhaseClient":
        reader, writer = await asyncio.open_unix_connection(path)
        return cls(reader, writer, session_id)

    async def set(self, name: str, expected_wait_ms: int | None = None) -> None:
        self.generation += 1
        request = {
            "schema_version": 1,
            "session_id": self.session_id,
            "generation": self.generation,
            "phase": name,
            "expected_wait_ms": expected_wait_ms,
        }
        self.writer.write((json.dumps(request, separators=(",", ":")) + "\n").encode())
        await self.writer.drain()
        raw = await asyncio.wait_for(self.reader.readline(), timeout=15.0)
        if not raw:
            raise ConnectionError("phase socket closed")
        ack = json.loads(raw)
        if ack.get("generation") != self.generation:
            raise RuntimeError(f"phase acknowledgement generation mismatch: {ack}")
        if not ack.get("accepted"):
            raise RuntimeError(ack.get("error") or "phase rejected")

    async def close(self) -> None:
        self.writer.close()
        try:
            await self.writer.wait_closed()
        except (BrokenPipeError, ConnectionError):
            pass


async def selector_exists(page: Any, selector: str) -> bool:
    return bool(await page.get_elements_by_css_selector(selector))


async def websocket_ready(page: Any) -> bool:
    return len((await live_state(page)).get("sequences", [])) >= 2


async def live_state(page: Any) -> dict[str, Any]:
    raw = await page.evaluate("() => window.__baeld ?? {reconnects:0,sequences:[]}")
    value = json.loads(raw)
    return value if isinstance(value, dict) else {"reconnects": 0, "sequences": []}


async def background_operations(page: Any, workload: str, base_url: str, session_id: str) -> int:
    if workload == "agent-dashboard":
        state = await http_json(f"{base_url}/api/state?session={urllib.parse.quote(session_id)}")
        return int(state.get("dashboard_requests", 0))
    if workload == "websocket":
        return len((await live_state(page)).get("sequences", []))
    return 0


async def verify(page: Any, workload: str, base_url: str, session_id: str) -> dict[str, Any]:
    if workload == "static":
        text = await page.evaluate("() => document.querySelector('#oracle')?.textContent ?? ''")
        return {"success": True} if text == "BAELD_STATIC_OK" else {"success": False, "failure": f"static oracle was {text}"}
    state = await http_json(f"{base_url}/api/state?session={urllib.parse.quote(session_id)}")
    if state.get("updates") == 1 and state.get("value") == "persisted":
        return {"success": True}
    return {"success": False, "failure": f"mutation oracle: {json.dumps(state, sort_keys=True)}"}


async def http_json(url: str, method: str = "GET") -> dict[str, Any]:
    def request() -> dict[str, Any]:
        with urllib.request.urlopen(urllib.request.Request(url, method=method), timeout=10) as response:
            return json.load(response)

    return await asyncio.to_thread(request)


async def wait_until(predicate: Callable[[], Awaitable[bool]], timeout: float) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if await predicate():
            return
        await asyncio.sleep(0.1)
    raise TimeoutError(f"condition did not become true within {timeout:.1f}s")


def sequence_gaps(values: list[int]) -> int:
    return sum(max(0, current - previous - 1) for previous, current in zip(values, values[1:]))


def required(env: os._Environ[str], name: str) -> str:
    value = env.get(name)
    if not value:
        raise RuntimeError(f"Missing {name}")
    return value


if __name__ == "__main__":
    asyncio.run(main())
