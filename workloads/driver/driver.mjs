import net from "node:net";
import readline from "node:readline";
import { chromium } from "playwright";

const env = process.env;
const sessionId = required("BAELD_SESSION_ID");
const socketPath = required("BAELD_SOCKET");
const cdpUrl = required("BAELD_CDP_URL");
const baseUrl = required("BAELD_BASE_URL");
const workload = required("BAELD_WORKLOAD");
const mechanism = required("BAELD_MECHANISM");
const waitMs = Number(required("BAELD_WAIT_MS"));
const settleMs = Number(required("BAELD_SETTLE_MS"));
let generation = 0;

const phase = await phaseClient(socketPath);
const started = performance.now();
let resumeLatencyMs = 0;
let browser;

try {
  await phase.set("starting");
  browser = await chromium.connectOverCDP(cdpUrl);
  const context = await browser.newContext({ viewport: { width: 1280, height: 720 } });
  const page = await context.newPage();
  const cdp = await context.newCDPSession(page);

  await fetch(`${baseUrl}/api/reset?session=${encodeURIComponent(sessionId)}`, {method:"POST"});
  await phase.set("navigating");
  await page.goto(`${baseUrl}/${workload}?session=${encodeURIComponent(sessionId)}`, {waitUntil:"domcontentloaded"});
  await phase.set("observing");
  await page.locator("h1").waitFor();
  if (workload === "websocket") await page.waitForFunction(() => window.__baeld?.sequences.length >= 2);

  await phase.set("waiting_for_model", waitMs);
  if (mechanism === "chrome-lifecycle-freeze") {
    await cdp.send("Page.setWebLifecycleState", {state:"frozen"});
  }
  await sleep(waitMs);

  const resumeStarted = performance.now();
  if (mechanism === "chrome-lifecycle-freeze") {
    await cdp.send("Page.setWebLifecycleState", {state:"active"});
  }
  await phase.set("acting");
  resumeLatencyMs = performance.now() - resumeStarted;

  if (workload !== "static") {
    await page.locator("#save").click();
  }
  await phase.set("verifying");
  const oracle = await verify(page);
  await phase.set("settling");
  await sleep(settleMs);
  const live = workload === "websocket"
    ? await page.evaluate(() => window.__baeld)
    : {reconnects:0, sequences:[]};
  await phase.set("finished");
  await context.close();

  const gaps = sequenceGaps(live.sequences ?? []);
  process.stdout.write(JSON.stringify({
    success: oracle.success && gaps === 0,
    latency_ms: performance.now() - started,
    resume_latency_ms: resumeLatencyMs,
    reconnects: live.reconnects ?? 0,
    sequence_gaps: gaps,
    failure: oracle.failure ?? (gaps ? `${gaps} websocket sequence gaps` : null)
  }));
} catch (error) {
  process.stdout.write(JSON.stringify({
    success:false,
    latency_ms:performance.now()-started,
    resume_latency_ms:resumeLatencyMs,
    reconnects:0,
    sequence_gaps:0,
    failure:error?.stack ?? String(error)
  }));
} finally {
  phase.close();
  if (browser) await browser.close().catch(() => {});
}

async function verify(page) {
  if (workload === "static") {
    const text = await page.locator("#oracle").textContent();
    return text === "BAELD_STATIC_OK" ? {success:true} : {success:false, failure:`static oracle was ${text}`};
  }
  const response = await fetch(`${baseUrl}/api/state?session=${encodeURIComponent(sessionId)}`);
  const state = await response.json();
  return state.updates === 1 && state.value === "persisted"
    ? {success:true}
    : {success:false, failure:`mutation oracle: ${JSON.stringify(state)}`};
}

async function phaseClient(path) {
  const socket = net.createConnection(path);
  await new Promise((resolve, reject) => { socket.once("connect", resolve); socket.once("error", reject); });
  const lines = readline.createInterface({input:socket, crlfDelay:Infinity});
  const pending = new Map();
  lines.on("line", line => {
    const ack = JSON.parse(line); const waiter = pending.get(ack.generation);
    if (!waiter) return;
    pending.delete(ack.generation);
    ack.accepted ? waiter.resolve(ack) : waiter.reject(new Error(ack.error ?? "phase rejected"));
  });
  return {
    set(name, expectedWaitMs) {
      generation++;
      const request = {
        schema_version:1,
        session_id:sessionId,
        generation,
        phase:name,
        expected_wait_ms:expectedWaitMs ?? null
      };
      return new Promise((resolve, reject) => {
        pending.set(generation, {resolve,reject});
        socket.write(JSON.stringify(request) + "\n");
      });
    },
    close() { lines.close(); socket.destroy(); }
  };
}

function sequenceGaps(values) {
  let gaps = 0;
  for (let i=1; i<values.length; i++) if (values[i] > values[i-1] + 1) gaps += values[i]-values[i-1]-1;
  return gaps;
}

function required(name) {
  if (!env[name]) throw new Error(`Missing ${name}`);
  return env[name];
}

function sleep(ms) { return new Promise(resolve => setTimeout(resolve, ms)); }
