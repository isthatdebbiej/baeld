# Baeld v0.1.0-preview

Baeld is an experimental Linux benchmark for browser-agent inference waits. This
preview publishes a reusable measurement harness and an unfavorable systems
tradeoff; it is not a production governor recommendation.

The subsequent v0.1 runtime work is tracked under `Unreleased` in the changelog. These notes remain the historical record for the already-published measurement preview and must not be read as the feature list for the current development branch.

## Result

On one Vultr 4-vCPU/16-GB Ubuntu 22.04 development VM with pinned Chromium:

- cgroup process-tree freeze reduced net measured CPU by about 5.5% on the
  controlled dashboard at a 5-second wait and 11.1% at a 10-second wait;
- the same mechanism failed every WebSocket task in the two follow-up gates,
  causing reconnects and numbered-event gaps;
- Chrome lifecycle freeze did not stop the dashboard's polling in this active
  headless-page setup and showed no credible CPU improvement;
- a 25% one-CPU quota was neutral because baseline Chromium consumption was
  already below the quota;
- 2-second waits showed no demonstrated complete-task saving.

These are development results with three or five paired blocks, one VM lifetime,
concurrency one, and controlled workloads. They are below Baeld's declared
20-pair headline threshold. Do not generalize the percentages to production.

## Included

- Linux x86-64 CLI archive and SHA-256 checksum.
- Raw events, captured environments, processed summaries, charts, and paired
  analysis for the five development runs.
- Reproducible controlled workloads and experiment configurations.

## Known limitations

- Linux cgroup v2 and systemd delegation are required.
- Cgroup freeze is unsafe for the included live WebSocket workload.
- Compatibility is reported as `failure-observed` or `no-failure-observed`;
  the latter does not mean safe.
- Memory reclamation, browser density, distributed scheduling, Kubernetes,
  GPU optimization, autonomous phase inference, and production readiness are
  explicitly out of scope.

The useful result is the compatibility boundary: suspending an entire browser
process tree can save CPU during long known waits, but the mechanism needs an
explicit suspend-safety contract and must not be enabled universally.
