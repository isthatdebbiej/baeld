# Limitations

- Baeld controls and measures local Linux Chromium process trees; it cannot control a remote provider's server-side resources.
- Health thresholds identify pressure and unresponsiveness but cannot prove application correctness.
- Cgroup freeze suspends browser-wide processes and broke every controlled WebSocket freeze test. It requires explicit or exact compatibility-record authorization.
- Stagehand's public calls do not expose a separate internal model boundary, so Stagehand receives supervision and cleanup but not automatic inference-wait suspension.
- Browser Use's native click helper did not dispatch the controlled Chrome 151 action; the deterministic benchmark workaround is not applied to user agents.
- Filtering changes network and rendering behavior and requires workload-specific verification.
- Persistent profiles contain identity state and must never be shared across tenants or published with diagnostics.
- Controlled workloads do not represent the full web. Results apply to pinned framework, browser, kernel, and host versions.
- Distributed placement and a hosted control plane remain outside v0.1.
