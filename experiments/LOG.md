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
