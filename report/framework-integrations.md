# Framework compatibility evidence

Baeld owns local Chromium and keeps framework processes outside the browser cgroup. Every integration must preserve phase acknowledgement, complete process membership, authoritative verification, bounded shutdown, and reproducibility.

## Playwright

The primary controlled suite uses pinned Playwright and Baeld-owned Chromium. It has passed the Ubuntu 24.04 benchmark and cleanup gates. Full process freeze consistently violated the WebSocket oracle; other tested policies preserved it.

## Stagehand

Stagehand 4.0.1 passed a 32-task deterministic local-CDP gate. Stagehand attached to Baeld-owned Chromium and did not launch an unaccounted browser. Its dashboard result did not demonstrate a complete-task improvement from freezing, and all frozen WebSocket tasks failed.

The public `@baeld/agent` package provides the same attachment and phase contract. Stagehand's public calls combine observation, inference, and browser interaction, so Baeld cannot safely suspend Chromium around the internal model call. Stagehand is supported for ownership, health, limits, telemetry, and cleanup; automatic inference-wait suspension requires an upstream hook and is not claimed.

## Browser Use

Browser Use 0.13.6 passed a 32-task deterministic attachment gate. All baseline WebSocket tasks passed and all frozen tasks failed. Native `Element.click()` did not dispatch the controlled button under Chrome 151, so the benchmark used element discovery followed by an evaluated click. That limitation is visible in the public integration guide and is not treated as stable native-action evidence.

The `baeld-agent` Python package provides browser attachment, phase APIs, and `BaeldPhaseModel`, which wraps Browser Use's `BaseChatModel.ainvoke` to expose the exact inference boundary. A provider-configured model-driven run remains pending.

## Remote browsers

A client-side Baeld runtime cannot control or directly measure a remote provider's Chromium process tree. Remote integrations are limited to phase compatibility, client telemetry, and end-to-end behavior unless Baeld runs inside the provider infrastructure.

## Support rule

Installation, navigation, or a successful CDP connection alone does not establish support. A framework becomes release-supported only after deterministic and real-agent gates pass correctness, accounting, concurrency, and cleanup requirements on a clean Ubuntu host.
