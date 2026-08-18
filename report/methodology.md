# Methodology

## Research question

During browser-agent inference waits, which mechanism—default Chromium behavior, Chromium-native page freezing, cgroup CPU throttling, or cgroup process-tree freezing—offers the best CPU/correctness tradeoff?

## Primary outcome

The primary outcome is CPU seconds consumed by the Chromium cgroup per successful complete task. The task window begins before observation and ends after action, authoritative verification, and a fixed three-second settling period. This captures work deferred until the browser resumes.

Chromium, Playwright driver, Baeld governor, and workload-server CPU are
reported separately. The primary net CPU numerator is Chromium + driver +
governor CPU; workload-server CPU is reported as an experimental control rather
than credited to a browser policy. Failed-attempt CPU remains in the numerator
and is never converted into resource savings.

Terminal schema-4 events also record cgroup CPU throttling, current and peak
memory, read/write bytes, and CPU/memory/I/O PSI diagnostics. These are
secondary diagnostics only: v0.1 does not claim memory or I/O optimization.

## Experimental controls

- Exact Chromium, Playwright, Rust, Node, kernel, VM, browser flags, and repository revision are recorded.
- Each measured task uses a fresh Chromium process and profile. Browser launch occurs before the primary accounting window, so startup cost is excluded and cache state is consistently cold.
- Mechanism order is randomized.
- Every terminal event records its concurrency and randomized block identity.
  Analysis never pools concurrency levels. The final study uses paired blocks
  and at least twenty repetitions for headline comparisons.
- Workload and coordinator CPUs are separated from Chromium CPUs using affinity during final runs.
- Default Chromium background throttling remains enabled in the primary experiment.

## Workload classification

- `static`: negative control.
- `normal-spa`: representative controlled workload and headline source.
- `agent-dashboard`: realism candidate with polling, JSON processing, sorting,
  formatting, DOM reconciliation, and local state; it becomes a headline source
  only after its behavior is validated against a real application trace.
- `noisy-stress`: intentionally favorable stress case, never a standalone headline source.
- `websocket`: correctness/failure control.

## Exclusions

Application failures are retained. A run may be marked infrastructure-invalid only when the workload server is unavailable, the browser never launches, the host loses required cgroup control, or the benchmark process is externally interrupted. Every exclusion and its raw artifact must be published.

CLI exit status distinguishes these categories. A completed benchmark with
application-policy failures exits successfully and records those failures.
Harness, host-control, launch, protocol, or cleanup errors return a failing exit
status.

Complete-task CPU remains primary. Browser CPU measured strictly from the
acknowledged `waiting_for_model` transition to the received `acting` transition
is diagnostic: it explains whether savings occur during the wait but cannot by
itself support a performance claim because work may shift after resume.
