import { chromium } from "playwright";
import { BaeldAgent } from "../../packages/js/src/index.js";

const base = required("BAELD_BASE_URL");
const session = required("BAELD_SESSION_ID");
const agent = await BaeldAgent.connect({ framework: "playwright", workload: "real-agent-gate" });
await agent.phase("starting");
const browser = await chromium.connectOverCDP(required("BAELD_CDP_URL"));
const page = browser.contexts()[0].pages()[0] ?? await browser.contexts()[0].newPage();
await fetch(`${base}/api/reset?session=${session}`, { method: "POST" });
await agent.phase("navigating");
await page.goto(`${base}/normal-spa?session=${session}`);
await agent.phase("observing");
await agent.waitingForModel(5_000);
const response = await fetch(`${required("OPENAI_BASE_URL").replace(/\/$/, "")}/chat/completions`, { method: "POST", headers: { authorization: `Bearer ${required("OPENAI_API_KEY")}`, "content-type": "application/json" }, body: JSON.stringify({ model: required("BAELD_MODEL"), temperature: 0, messages: [{ role: "system", content: "Return only the CSS selector for the save button." }, { role: "user", content: "The page contains <button id=save>Save</button>." }] }) });
if (!response.ok) throw new Error(`model endpoint returned ${response.status}: ${await response.text()}`);
const selector = (await response.json()).choices?.[0]?.message?.content?.trim();
await agent.acting();
if (selector !== "#save") throw new Error(`model returned unexpected selector ${selector}`);
await page.locator(selector).click();
await agent.verify();
const state = await (await fetch(`${base}/api/state?session=${session}`)).json();
if (state.updates !== 1 || state.value !== "persisted") throw new Error(`oracle failed: ${JSON.stringify(state)}`);
await agent.phase("settling");
await new Promise(resolve => setTimeout(resolve, 3000));
await agent.finish();
agent.close();

function required(name) { const value = process.env[name]; if (!value) throw new Error(`Missing ${name}`); return value; }
