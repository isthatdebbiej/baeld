# Framework integration plan

## Tested boundary

Baeld's current evidence comes from a pinned Playwright driver controlling a
locally launched Chromium process tree. The tasks use deterministic observation,
simulated model waits, scripted actions, and authoritative workload oracles.
They do not invoke an LLM and do not use Stagehand, Browser Use, Browserbase, or
another browser-agent framework.

This distinction matters. The result establishes that Baeld can account for and
control a local Chromium tree. It does not establish that a framework exposes
reliable wait boundaries, preserves browser ownership, tolerates suspension, or
avoids work outside the browser cgroup.

## Common adapter contract

Do not add framework-specific policy logic to the Rust governor. Add thin
drivers that emit the existing versioned phase messages and use the existing
workload oracles. Every adapter must report:

- framework and version;
- local or remote browser ownership;
- browser executable and launch flags when observable;
- model provider, model, and whether inference is real or a deterministic stub;
- phase-boundary source and generation;
- framework, model-client, browser, Baeld, and workload-server CPU separately;
- task success, duplicate mutations, reconnects, sequence gaps, and recovery;
- any browser descendants outside the session cgroup.

The first comparison uses a deterministic model stub with fixed 5- and 10-second
delays. This isolates framework integration from model latency, cost, and output
variance. A small real-model confirmation follows only after the adapter passes
correctness and accounting checks.

## Validation order

### 1. Stagehand with local Chromium

Stagehand is the closest next step because its browser session can be exercised
through Playwright/CDP while Baeld owns the local Chromium process tree.

Start with 32 tasks: dashboard and WebSocket, baseline and cgroup freeze,
5-second waits, concurrency one, four paired blocks. Gate expansion on complete
process membership and exact oracle results. Then run the four-mechanism matrix
at concurrency one and five. Record Stagehand's own Node CPU outside the browser
cgroup.

Failure conditions include Stagehand launching an unaccounted browser, hiding
the inference boundary, losing its CDP session after thaw, or changing the
authoritative action semantics.

### 2. Browser Use with local Chromium

Use Browser Use's supported local/custom-browser attachment path and keep its
Python process outside the browser cgroup. Repeat the same 32-task gate before
expansion. Pay particular attention to watchdogs, background tabs, browser
relaunch, event-loop work during waits, and child processes created after
attachment.

If Browser Use cannot attach to a Baeld-owned browser without altering normal
behavior, record that as an integration limitation; do not move the Python
agent into the browser cgroup merely to make accounting convenient.

### 3. Stagehand plus Browserbase

Browserbase is a remote-browser comparison, not a target for the local cgroup
governor. A client-side Baeld process cannot freeze or throttle Browserbase's
remote Chromium tree and cannot directly measure its CPU counters.

Use the same tasks and timing protocol to test semantic compatibility and
end-to-end latency. Report client CPU and provider-visible metrics separately.
Do not compare provider CPU savings unless Browserbase exposes equivalent
server-side accounting or runs Baeld within its infrastructure.

### 4. Small real-model confirmation

After both local adapters pass deterministic gates, run a bounded confirmation
with one pinned model and recorded prompts: five paired blocks for dashboard and
WebSocket at one wait regime. Preserve model failures and token usage. This is a
compatibility check, not the primary performance dataset, because inference
variance weakens pairing and increases cost.

## Stop/go rules

- Stop an adapter if browser descendants escape accounting or authoritative
  verification cannot be preserved.
- Do not expand a mechanism that fails any WebSocket gate into a headline
  performance run; retain the failure as compatibility evidence.
- Do not claim framework support from successful installation or navigation.
  Support requires phase acknowledgement, complete accounting, correctness,
  cleanup, and reproducibility on a clean host.
- Keep cgroup freeze opt-in. The Playwright pilot's 3.5–8.0% dashboard CPU
  reduction came with 60 failures in 60 WebSocket attempts.

## Lean implementation sequence

1. Extract the current Playwright driver boundary into a documented adapter
   interface without changing the Rust policy engine or event schema.
2. Add the Stagehand deterministic gate and publish its raw result even if it
   fails.
3. Add Browser Use only after the Stagehand adapter proves the contract is not
   Playwright-specific.
4. Treat Browserbase as a remote compatibility track.
5. Freeze dependency versions and expand repetitions only after each 32-task
   gate passes infrastructure and accounting checks.

This sequence tests the riskiest assumptions cheaply. It avoids spending model
tokens or hundreds of VM tasks on an adapter whose browser tree is not actually
under Baeld's control.
