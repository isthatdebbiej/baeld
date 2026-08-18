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
