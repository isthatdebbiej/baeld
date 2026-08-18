import net from "node:net";
import readline from "node:readline";
import { Stagehand, localBrowser } from "@browserbasehq/stagehand";

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

if (!["baseline", "cgroup-freeze-500ms"].includes(mechanism)) {
  throw new Error(`Stagehand gate does not support mechanism ${mechanism}`);
}

const phase = await phaseClient(socketPath);
const started = performance.now();
let resumeLatencyMs = 0;
let stagehand;
let browserHandle;

try {
  await phase.set("starting");
  browserHandle = await localBrowser.connect({ cdpUrl });
  stagehand = await Stagehand.create({
    browser: browserHandle,
    logging: { level: "off", format: "json" },
  });
  const page = await stagehand.browser.context.newPage();

  await fetch(`${baseUrl}/api/reset?session=${encodeURIComponent(sessionId)}`, { method: "POST" });
  await phase.set("navigating");
  await page.goto(`${baseUrl}/${workload}?session=${encodeURIComponent(sessionId)}`);
  await phase.set("observing");
  if (!(await page.waitForSelector("h1", { timeout: 15_000 }))) {
    throw new Error("Stagehand did not observe the workload heading");
  }
  if (workload === "websocket") {
    await waitUntil(async () => (await liveState(page)).sequences.length >= 2, 15_000);
  }

  const backgroundBefore = await backgroundOperations(page);
  await phase.set("waiting_for_model", waitMs);
  await sleep(waitMs);

  const resumeStarted = performance.now();
  await phase.set("acting");
  resumeLatencyMs = performance.now() - resumeStarted;
  const backgroundAfter = await backgroundOperations(page);

  if (workload !== "static") await page.locator("#save").click();
  await phase.set("verifying");
  const oracle = await verify(page);
  await phase.set("settling");
  await sleep(settleMs);
  const live = workload === "websocket"
    ? await liveState(page)
    : { reconnects: 0, sequences: [] };
  await phase.set("finished");

  const gaps = sequenceGaps(live.sequences ?? []);
  process.stdout.write(JSON.stringify({
    success: oracle.success && gaps === 0,
    latency_ms: performance.now() - started,
    resume_latency_ms: resumeLatencyMs,
    reconnects: live.reconnects ?? 0,
    sequence_gaps: gaps,
    background_operations: Math.max(0, backgroundAfter - backgroundBefore),
    failure: oracle.failure ?? (gaps ? `${gaps} websocket sequence gaps` : null),
  }));
} catch (error) {
  process.stdout.write(JSON.stringify({
    success: false,
    latency_ms: performance.now() - started,
    resume_latency_ms: resumeLatencyMs,
    reconnects: 0,
    sequence_gaps: 0,
    background_operations: 0,
    failure: error?.stack ?? String(error),
  }));
} finally {
  phase.close();
  await closeBounded(stagehand ?? browserHandle, 3_000);
  await new Promise(resolve => process.stdout.write("", resolve));
  process.exit(0);
}

async function verify(page) {
  if (workload === "static") {
    const text = await page.locator("#oracle").textContent();
    return text === "BAELD_STATIC_OK"
      ? { success: true }
      : { success: false, failure: `static oracle was ${text}` };
  }
  const response = await fetch(`${baseUrl}/api/state?session=${encodeURIComponent(sessionId)}`);
  const state = await response.json();
  return state.updates === 1 && state.value === "persisted"
    ? { success: true }
    : { success: false, failure: `mutation oracle: ${JSON.stringify(state)}` };
}

async function liveState(page) {
  return page.evaluate(() => window.__baeld ?? { reconnects: 0, sequences: [] });
}

async function phaseClient(path) {
  const socket = net.createConnection(path);
  await new Promise((resolve, reject) => {
    socket.once("connect", resolve);
    socket.once("error", reject);
  });
  const lines = readline.createInterface({ input: socket, crlfDelay: Infinity });
  const pending = new Map();
  lines.on("line", line => {
    let ack;
    try { ack = JSON.parse(line); }
    catch (error) { rejectPending(new Error(`invalid phase acknowledgement: ${error}`)); return; }
    const waiter = pending.get(ack.generation);
    if (!waiter) return;
    pending.delete(ack.generation);
    clearTimeout(waiter.timeout);
    ack.accepted ? waiter.resolve(ack) : waiter.reject(new Error(ack.error ?? "phase rejected"));
  });
  socket.on("error", rejectPending);
  socket.on("close", () => rejectPending(new Error("phase socket closed")));

  function rejectPending(error) {
    for (const waiter of pending.values()) {
      clearTimeout(waiter.timeout);
      waiter.reject(error);
    }
    pending.clear();
  }

  return {
    set(name, expectedWaitMs) {
      generation++;
      const request = {
        schema_version: 1,
        session_id: sessionId,
        generation,
        phase: name,
        expected_wait_ms: expectedWaitMs ?? null,
      };
      return new Promise((resolve, reject) => {
        const timeout = setTimeout(() => {
          pending.delete(generation);
          reject(new Error(`phase ${name} generation ${generation} timed out`));
        }, 15_000);
        pending.set(generation, { resolve, reject, timeout });
        socket.write(JSON.stringify(request) + "\n");
      });
    },
    close() {
      rejectPending(new Error("phase client closed"));
      lines.close();
      socket.destroy();
    },
  };
}

function sequenceGaps(values) {
  let gaps = 0;
  for (let i = 1; i < values.length; i++) {
    if (values[i] > values[i - 1] + 1) gaps += values[i] - values[i - 1] - 1;
  }
  return gaps;
}

async function backgroundOperations(page) {
  if (workload === "agent-dashboard") {
    const response = await fetch(`${baseUrl}/api/state?session=${encodeURIComponent(sessionId)}`);
    return (await response.json()).dashboard_requests ?? 0;
  }
  if (workload === "websocket") return (await liveState(page)).sequences.length;
  return 0;
}

async function waitUntil(predicate, timeoutMs) {
  const deadline = performance.now() + timeoutMs;
  while (performance.now() < deadline) {
    if (await predicate()) return;
    await sleep(100);
  }
  throw new Error(`condition did not become true within ${timeoutMs}ms`);
}

function required(name) {
  if (!env[name]) throw new Error(`Missing ${name}`);
  return env[name];
}

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function closeBounded(resource, timeoutMs) {
  if (!resource) return;
  await Promise.race([
    resource.close().catch(() => {}),
    sleep(timeoutMs),
  ]);
}
