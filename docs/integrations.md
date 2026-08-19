# Framework integrations

All adapters use the same versioned phase protocol. Framework code attaches to the Chromium endpoint; policy remains in Rust.

| Integration | Deterministic attachment | Runtime package | Real-agent release gate |
|---|---:|---:|---:|
| Playwright 1.62.1 | Passed | `@baeld/agent` | Pending clean Linux run |
| Stagehand 4.0.1 | Passed | `@baeld/agent` | Supervision only; exact internal model boundary unavailable |
| Browser Use 0.13.6 | Passed with click limitation | `baeld-agent` | Pending provider-configured run |

“Passed attachment” means Baeld owned Chromium, the framework attached through CDP, phase acknowledgements worked, the oracle ran, and cleanup completed. It does not mean model-driven actions were evaluated.

Emit `starting`, `navigating`, `observing`, `waiting_for_model`, `acting`, `verifying`, `settling`, and `finished` with increasing generations. Report a critical live connection when interrupting background traffic could invalidate the task.

Playwright applications can wrap their model call with `withModelWait`. Browser Use applications should wrap `ChatOpenAI` or another `BaseChatModel` with `BaeldPhaseModel`, which emits phases around `ainvoke` without changing model responses.

Stagehand's public `act()` and `observe()` combine browser observation and model work. Baeld can own, monitor, limit, and clean up Stagehand sessions, but cannot safely freeze inside those calls without an upstream model-boundary hook. Do not surround an entire `act()` with a frozen wait phase: the browser may need to act before the call returns.

In the controlled Chrome 151 gate, Browser Use 0.13.6 discovered the target element but native `Element.click()` did not dispatch it. The deterministic benchmark used `Page.evaluate()`. Baeld does not generalize that workaround to user agents.

Remote providers can use phase adapters and client telemetry, but local Baeld cannot control or measure their server-side Chromium cgroups.

The provider-configured release gate reads `OPENAI_BASE_URL`, `OPENAI_API_KEY`, and a pinned `BAELD_MODEL` from the environment. It runs five controlled blocks for each integration and never writes credentials, prompts, or model responses to released artifacts. Run it with `scripts/run-real-agent-gates.sh` on the declared Linux host.
