# Changelog

## Unreleased

- Add `baeld run`, session inspection, stale cleanup, health monitoring, limits, admission control, and OTLP/JSONL telemetry.
- Add selectable observe, safe, adaptive, and explicit policy modes.
- Add npm Playwright/Stagehand and PyPI Browser Use adapter packages.
- Add measured filtering interfaces and ephemeral, persistent, and warm session configuration.
- Replace internal planning prose with user configuration, policy, integration, architecture, telemetry, troubleshooting, and benchmark guides.
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
