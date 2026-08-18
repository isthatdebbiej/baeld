# Experiment ledger

Every experiment entry must include its hypothesis, exact command, result, decision, and artifact directory. Results are never overwritten.

## E000 — Initial baseline plan

- Hypothesis: At least one suspension mechanism changes complete-task CPU or correctness relative to default Chromium.
- Command: `just smoke`
- Result: Superseded by E001 after the delegated Vultr host was provisioned.
- Decision: Preserve the original gate and run it before the larger matrix.
- Artifacts: None.

## E001 — Vultr smoke gate

- Hypothesis: All four mechanisms can complete the controlled tasks and clean
  up their browser process trees.
- Command: `scripts/run-scoped.sh smoke`
- Result: 11 of 12 tasks succeeded. Cgroup freeze failed the WebSocket task
  with one reconnect and four numbered-event gaps. The strict smoke command
  correctly refused to launch the larger gate. Host diagnostics, unprivileged
  Chromium sandboxing, and cgroup freeze/thaw passed.
- Decision: Treat the WebSocket failure as a mechanism-level correctness
  distinction worth reproducing, not as permission to weaken the oracle.
  Continue with the development matrix by invoking `bench` directly so all
  failures remain in the dataset.
- Artifacts: Local ignored directory
  `results/1787012108-smoke-9025976d/`.

## E002 — Vultr cloud development gate

- Hypothesis: A longer model wait will expose a material CPU reduction from at
  least one mechanism without sacrificing correctness.
- Command: `scripts/run-scoped.sh bench --config experiments/cloud-gate.toml`
- Result: All 48 tasks produced terminal records. No mechanism demonstrated a
  credible CPU improvement on `normal-spa` with three repetitions. Cgroup
  freeze failed all six WebSocket tasks, causing six reconnects and 56 total
  numbered-event gaps. The other mechanisms passed every WebSocket task.
- Decision: Do not claim a CPU win. Investigate why baseline wait CPU is low,
  improve workload realism without converting the benchmark into an artificial
  stress test, and retain cgroup freeze as a negative correctness control.
- Artifacts: Local ignored directory
  `results/1787012208-cloud-development-gate-3002fb14/`; interpretation in
  `report/results.md`.

## E003 — Agent workload and wait-window accounting gate

- Hypothesis: Direct wait-window accounting and a non-synthetic dashboard will
  reveal whether the earlier neutral result was caused by whole-task dilution
  or by genuinely negligible background work.
- Command: `scripts/run-scoped.sh bench --config experiments/agent-workload-gate.toml`
- Result: All 80 tasks succeeded. At 5 seconds, cgroup freeze reduced net CPU
  per successful dashboard task by 5.5% (paired mean -0.0889 seconds; five-pair
  bootstrap 95% interval [-0.1491, -0.0265]). The effect did not produce a
  demonstrated complete-task saving at 2 seconds. Native lifecycle freeze and
  the 25% CPU quota were neutral. This matrix excluded the WebSocket workload;
  E001 and E002 remain the applicable correctness evidence for cgroup freeze.
- Decision: Do not run the full pilot yet. First test a 10-second wait, diagnose
  native lifecycle behavior, add the WebSocket control back into the focused
  matrix, and validate the dashboard against one pinned real application trace.
- Artifacts: Local ignored directory
  `results/1787015970-agent-workload-gate-77dabfb1/`; interpretation in
  `report/results.md`.

## E004 — Long-wait correctness gate

- Hypothesis: A 10-second wait increases complete-task savings while the
  WebSocket control exposes the associated compatibility boundary.
- Command: `scripts/run-scoped.sh bench --config experiments/long-wait-correctness-gate.toml`
- Result: All 24 dashboard tasks succeeded. At 10 seconds, cgroup freeze reduced
  dashboard net CPU by 11.1%. All six cgroup-freeze WebSocket tasks failed,
  with six reconnects and 154 sequence gaps. The other 18 WebSocket tasks
  passed. The dashboard operation counter was invalid due to a misplaced
  initializer and is excluded; CPU and correctness accounting were unaffected.
- Decision: Preserve the CPU and WebSocket results, fix the diagnostic counter,
  and rerun only the 24 dashboard cells.
- Artifacts: `results/1787017302-long-wait-correctness-gate-c8778ac1/`.

## E005 — Corrected dashboard freeze diagnostic

- Hypothesis: Server-side request counts will distinguish mechanisms that truly
  stop dashboard polling from mechanisms whose CPU differences are noise.
- Command: `scripts/run-scoped.sh bench --config experiments/dashboard-freeze-diagnostic.toml`
- Result: All 24 tasks succeeded. Baseline, Chrome lifecycle freeze, and CPU
  quota allowed exactly 10 refreshes at 5 seconds and 20 at 10 seconds. Cgroup
  freeze allowed means of 1.67 and 1.0. Chrome lifecycle freeze therefore did
  not suspend polling in this active headless-page setup.
- Decision: Stop before the concurrency pilot. Reframe Baeld around profiling
  and explicit suspend compatibility; retain cgroup freeze as opt-in and never
  as an inferred or universal policy.
- Artifacts: `results/1787017971-dashboard-freeze-diagnostic-c3dd78a8/`;
  interpretation in `report/results.md`.

## E006 — Schema-4 smoke on the resized development host

- Hypothesis: Schema 4 preserves the four-mechanism smoke behavior while
  recording concurrency, block identity, and secondary cgroup diagnostics.
- Command: `scripts/run-scoped.sh smoke --output results`
- Result: 11 of 12 tasks passed. The cgroup-freeze WebSocket task failed with
  one reconnect and three sequence gaps; the other WebSocket mechanisms
  passed. Terminal events contained the new schema-4 concurrency, block,
  memory, I/O, throttling, and PSI fields. Analysis regenerated successfully,
  and no Baeld Chromium process remained afterward.
- Decision: Proceed to the bounded concurrency accounting gate, not the full
  pilot. Retain Ubuntu 22.04 classification as development-only.
- Artifacts: `results/1787023280-smoke-ad754257/`.

## E007 — Schema-4 bounded concurrency gate

- Hypothesis: Concurrency-one and concurrency-five measurements remain
  separate and are paired by randomized block identity.
- Command: `scripts/run-scoped.sh bench --config experiments/concurrency-gate.toml --output results`
- Result: All 48 tasks produced terminal events. The analyzers emitted distinct
  concurrency-one and concurrency-five cells with two `block_id` pairs each.
  Cgroup freeze failed all 12 WebSocket tasks, producing 12 reconnects and 39
  sequence gaps. Baseline, lifecycle freeze, and CPU quota passed all 36 of
  their WebSocket tasks. Normal-SPA net CPU differences were between roughly
  -0.03 and +0.02 seconds and are two-pair development noise.
- Decision: The concurrency-accounting repair is validated. Do not enlarge the
  Ubuntu 22.04 matrix; move the next evidence run to a clean Ubuntu 24.04 host.
- Artifacts: `results/1787023423-concurrency-development-gate-dd0e305b/`.

## E008 — Clean Ubuntu 24.04 smoke gate

- Hypothesis: The declared final operating-system family can run pinned
  Chromium sandboxed, exercise delegated cgroups, and reproduce the development
  compatibility distinction.
- Command: `scripts/run-scoped.sh doctor` followed by
  `scripts/run-scoped.sh smoke --output results`.
- Result: `doctor` passed on Ubuntu 24.04.4, kernel 6.8, 8 vCPUs, and 32 GB RAM.
  Ubuntu's AppArmor user-namespace restriction initially blocked downloaded
  Chromium; a root-owned profile allowlisting only the pinned Playwright binary
  restored the sandbox while the global restriction remained enabled. Smoke
  completed 11 of 12 application tasks. Cgroup freeze caused one reconnect and
  four WebSocket sequence gaps; the other mechanisms passed their WebSocket
  tasks.
- Decision: The clean host is valid for a pilot. Keep the AppArmor profile in
  setup and never substitute `--no-sandbox` or a global sysctl disable.
- Artifacts: `results/1787024847-smoke-a3fb2a58/`.
