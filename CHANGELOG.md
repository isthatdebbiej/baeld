# Changelog

## Unreleased

- Record concurrency and randomized block identity in terminal benchmark events.
- Prevent the analyzers from pooling concurrency 1, 5, and 10 results.
- Pair schema-4 measurements by block identity and retain failed-attempt CPU.
- Report driver, governor, workload-server CPU, and host steal separately.

## 0.1.0-preview — 2026-08-18

- Added complete-task and model-wait CPU accounting.
- Added controlled SPA, dashboard, stress, static, and WebSocket workloads.
- Added conservative compatibility assessment and paired bootstrap analysis.
- Retained the observed cgroup-freeze WebSocket failures as first-class results.

This release is a Linux experimental research preview. It does not promise a
stable library API or production-safe automatic suspension.
