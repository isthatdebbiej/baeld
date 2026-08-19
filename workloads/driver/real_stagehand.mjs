import { Stagehand, localBrowser } from "@browserbasehq/stagehand";
import { connectStagehand } from "../../packages/js/src/index.js";

const base = required("BAELD_BASE_URL");
const session = required("BAELD_SESSION_ID");
const { stagehand, agent } = await connectStagehand({ Stagehand, localBrowser }, { workload: "real-agent-gate", stagehandOptions: { model: { modelName: `openai/${required("BAELD_MODEL")}`, apiKey: required("OPENAI_API_KEY"), baseUrl: required("OPENAI_BASE_URL") } } });
await agent.phase("starting");
const page = await stagehand.browser.context.newPage();
await fetch(`${base}/api/reset?session=${session}`, { method: "POST" });
await agent.phase("navigating");
await page.goto(`${base}/normal-spa?session=${session}`);
await agent.phase("observing");
// Stagehand does not expose a separate inference boundary. Keep Chromium active.
await stagehand.act("Click the Save button exactly once");
await agent.phase("acting");
await agent.verify();
const state = await (await fetch(`${base}/api/state?session=${session}`)).json();
if (state.updates !== 1 || state.value !== "persisted") throw new Error(`oracle failed: ${JSON.stringify(state)}`);
await agent.phase("settling");
await new Promise(resolve => setTimeout(resolve, 3000));
await agent.finish();
agent.close();

function required(name) { const value = process.env[name]; if (!value) throw new Error(`Missing ${name}`); return value; }
