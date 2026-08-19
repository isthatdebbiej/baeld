# Baeld

Baeld is an open-source Linux runtime for supervising and optimizing Chromium sessions used by browser agents. It owns the complete browser process tree, understands explicit agent phases, bounds resource use, cleans up orphaned processes, and records enough evidence to explain what happened.

Baeld works with Playwright, Stagehand, and Browser Use. It does not replace those frameworks or decide what an agent should click.

> Baeld is a v0.1 preview. Use it on isolated workers and controlled workloads. Full process freezing is never assumed safe.

## Why Baeld

Browser agents alternate between expensive navigation, observation, model waits, actions, and verification. Baeld connects those phases to Linux process-tree health so it can:

- account for every Chromium renderer, worker, GPU, network, and utility process;
- limit memory and process growth per session;
- stagger navigation and resume bursts across concurrent agents;
- detect stuck or degraded sessions;
- restore unrestricted resources before an agent acts;
- terminate abandoned process trees when a controller exits;
- compare policies without hiding deferred work or correctness failures.

## Requirements

- Ubuntu 24.04 x86-64, systemd, and cgroup v2;
- delegated `cpu`, `memory`, `io`, and `pids` controllers;
- unprivileged Chromium sandboxing;
- Rust 1.85;
- Node 22.18+ and Bun 1.3.14 for JavaScript adapters;
- Python 3.12+ for Browser Use.

Windows and macOS are development hosts only. Baeld refuses to run Chromium as root or with `--no-sandbox`.

## Install and verify

```bash
git clone https://github.com/isthatdebbiej/baeld.git
cd baeld
scripts/setup-ubuntu.sh
cargo build --release --locked
scripts/run-scoped.sh doctor
cp baeld.example.toml baeld.toml
```

## Five-minute Playwright example

```bash
bun add @baeld/agent playwright
```

```js
import { connectPlaywright, installFiltering, withModelWait } from "@baeld/agent";
import { chromium } from "playwright";

const { browser, agent } = await connectPlaywright({ chromium }, { workload: "support-agent" });
const context = browser.contexts()[0];
const page = context.pages()[0] ?? await context.newPage();

await agent.phase("starting");
await agent.phase("navigating");
await installFiltering(page);
await page.goto("https://example.com");
await agent.phase("observing");
const decision = await withModelWait(agent, 5_000, () => callYourModel());
await page.getByRole("button", { name: decision.button }).click();
await agent.verify();
// Verify the application outcome here.
await agent.phase("settling");
await agent.finish();
agent.close();
await browser.close();
```

```bash
scripts/run-scoped.sh run --config baeld.toml -- bun run agent.js
```

The child receives `BAELD_CDP_URL`, `BAELD_PHASE_SOCKET`, and `BAELD_SESSION_ID`. An uninstrumented command still receives ownership, limits, monitoring, and cleanup; it does not receive phase-aware optimization.

## Stagehand

```js
import { Stagehand, localBrowser } from "@browserbasehq/stagehand";
import { connectStagehand } from "@baeld/agent";

const { stagehand, agent } = await connectStagehand({ Stagehand, localBrowser });
await agent.phase("starting");
// Use stagehand.browser and emit the remaining phases around your workflow.
```

Stagehand local attachment requires Chromium extensions and a localhost CDP origin. Enable `allow_extensions = true` only for this integration.

## Browser Use

```bash
pip install baeld-agent browser-use
```

```python
from baeld_agent import BaeldPhaseModel, connect_browser_use
from browser_use import Agent, ChatOpenAI

browser, agent = await connect_browser_use()
await agent.phase("starting")
model = BaeldPhaseModel(ChatOpenAI(model="your-model"), agent)
runner = Agent(task="...", llm=model, browser_session=browser)
# Navigate and emit observing before runner.run(); model inference phases are wrapped.
```

Browser Use 0.13.6 has a documented native-click limitation in the controlled gate; see the [compatibility guide](docs/integrations.md).

## Policy modes

- `observe` records health and guarantees cleanup without performance intervention.
- `safe` adds conservative throttling and admission control; it never freezes the process tree.
- `adaptive` uses wait duration and connection signals inside the configured safety boundary. Unknown workloads are throttled, not frozen.
- `explicit` applies the mechanism selected by the user.

Cgroup freeze interrupted every controlled WebSocket session tested. Select it only after validating the exact workload. See [policies](docs/policies.md) and [experimental results](report/results.md).

## Commands

```bash
baeld run --config baeld.toml -- <agent command>
baeld sessions [--json]
baeld inspect <session-id> [--json]
baeld cleanup
baeld doctor
baeld smoke
baeld bench --config experiments/pilot.toml
baeld summarize results/<run-id>
```

## Documentation

- [Configuration](docs/configuration.md)
- [Policies and safety](docs/policies.md)
- [Framework integrations](docs/integrations.md)
- [Architecture](docs/architecture.md)
- [Telemetry](docs/telemetry.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Benchmark reproduction](docs/benchmarks.md)
- [Security](SECURITY.md)
- [Contributing](CONTRIBUTING.md)

## Current evidence

Controlled Ubuntu 24.04 experiments found that complete process-tree freezing reduced browser work during long, correctly signaled waits, but broke the live WebSocket oracle in every tested Playwright, Stagehand, and Browser Use freeze cell. Chromium lifecycle freeze and the tested CPU quotas were mostly neutral because default Chromium activity was already low.

Baeld therefore profiles and supervises first. It does not describe suspension as universally beneficial, claim memory reclamation, or infer production safety from successful installation.

## License

Licensed under either Apache-2.0 or MIT, at your option.
