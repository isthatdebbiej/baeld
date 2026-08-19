# Architecture

```text
Playwright / Stagehand / Browser Use
                │ phase protocol + CDP
                ▼
          Baeld runtime
 policy · admission · health · telemetry · cleanup
                │
                ▼
      Chromium process-tree cgroup
                │
                ▼
 Linux cgroup v2 and /proc
```

`baeld run` creates a delegated child cgroup before Chromium starts. The agent stays outside it and receives a localhost CDP endpoint plus Unix phase socket.

Runtime state is written atomically beneath `$XDG_STATE_HOME/baeld` or `~/.local/state/baeld`. Machine-wide permits coordinate expensive phases; dead owners are removed using `/proc` liveness.

Health progresses through `starting`, `healthy`, `degraded`, `stuck`, `terminating`, and `cleaned`. Repeated pressure produces `degraded`; CDP loss and phase timeout produce `stuck`. v0.1 terminates a stuck command and guarantees cleanup instead of attempting application-specific recovery.
