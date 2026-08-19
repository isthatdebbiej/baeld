#!/usr/bin/env python3
import asyncio
import os
import urllib.parse
import urllib.request
import json

from baeld_agent import BaeldPhaseModel, connect_browser_use
from browser_use import Agent, ChatOpenAI

def required(name: str) -> str:
    value = os.environ.get(name)
    if not value: raise RuntimeError(f"Missing {name}")
    return value

async def main() -> None:
    base, session = required("BAELD_BASE_URL"), required("BAELD_SESSION_ID")
    browser, phase = await connect_browser_use()
    await phase.phase("starting")
    page = await browser.new_page()
    urllib.request.urlopen(urllib.request.Request(f"{base}/api/reset?session={urllib.parse.quote(session)}", method="POST"), timeout=10).read()
    await phase.phase("navigating")
    await page.goto(f"{base}/normal-spa?session={urllib.parse.quote(session)}")
    await phase.phase("observing")
    model = ChatOpenAI(model=required("BAELD_MODEL"), api_key=required("OPENAI_API_KEY"), base_url=required("OPENAI_BASE_URL"), temperature=0)
    history = await Agent(task="Click the Save button exactly once, then finish.", llm=BaeldPhaseModel(model, phase), browser_session=browser, use_vision=False, max_actions_per_step=1).run(max_steps=4)
    if history.has_errors(): raise RuntimeError(f"Browser Use agent errors: {history.errors()}")
    if phase.previous_phase != "acting": await phase.phase("acting")
    await phase.verify()
    state = json.loads(urllib.request.urlopen(f"{base}/api/state?session={urllib.parse.quote(session)}", timeout=10).read())
    if state.get("updates") != 1 or state.get("value") != "persisted": raise RuntimeError(f"oracle failed: {state}")
    await phase.phase("settling"); await asyncio.sleep(3); await phase.finish(); await phase.close()

asyncio.run(main())
