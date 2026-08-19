# Results

## Vultr development gate — 2026-08-18 UTC

This is a development result, not a publication-quality benchmark. It was run
on Ubuntu 22.04.5 rather than the planned Ubuntu 24.04 final environment, with
three repetitions per cell and concurrency one. It is useful for falsifying
the premise and validating the harness; it is not powered for performance
claims.

Environment:

- Vultr KVM, AMD EPYC-Turin, 4 logical CPUs exposed to the benchmark.
- 16 GB RAM, Linux 5.15.0-187-generic, cgroup v2.
- Chrome for Testing 151.0.7922.34 and Playwright 1.62.1.
- Baeld commit `20dd4f5f80ee7f222e810986625178c212b2ac85`.
- Waits of 2 seconds and 5 seconds; `normal-spa` and `websocket` workloads.
- Baseline, Chrome lifecycle freeze, 25% CPU quota, and cgroup freeze after
  500 ms; three randomized repetitions per cell (48 tasks total).

### CPU result: no demonstrated improvement

All mechanisms completed all six `normal-spa` tasks. At the 5-second wait,
browser CPU seconds per successful task were 0.8722 for baseline, 0.8435 for
cgroup freeze, 0.8728 for Chrome lifecycle freeze, and 0.8768 for CPU quota.
Including browser, driver, and governor CPU produced 1.5112, 1.4760, 1.5179,
and 1.5235 seconds respectively.

The apparent cgroup-freeze reduction is approximately 3.3% for browser CPU
and 2.3% for net measured CPU. It is not a finding: there are only three
samples, the bootstrap intervals overlap, and the 2-second result moves in the
opposite direction for browser CPU. Chrome lifecycle freeze and CPU quota also
show no credible CPU improvement over baseline. Default Chromium behavior was
already inexpensive during these controlled waits.

### Correctness result: process-tree freeze breaks the WebSocket workload

Cgroup freeze failed all six WebSocket tasks. Each block caused one reconnect:
the 2-second waits lost 10 numbered events in total and the 5-second waits lost
46. Baseline, Chrome lifecycle freeze, and CPU quota passed all 18 corresponding
WebSocket tasks with no reconnects or sequence gaps.

This distinction is repeatable in the development run, but it is specific to
the controlled heartbeat/reconciliation oracle. It supports treating cgroup
freeze as unsafe for live real-time sessions; it does not establish a general
failure rate for production sites.

### Data status

The raw development artifacts are retained locally under
`results/1787012208-cloud-development-gate-3002fb14/` and contain 48 terminal
task events, the captured environment, processed summary, and chart. They are
intentionally ignored by Git under the development-data policy. Publication
data will be generated on the declared final environment and attached to a
versioned release. The environment record reports Rust as unavailable because
the transient system service did not inherit Cargo's PATH; the compiled binary
was built with the repository-pinned Rust 1.85 toolchain.

## Agent-workload development gate — 2026-08-18 UTC

This follow-up used the same Vultr host and pinned browser, added direct browser
CPU accounting for the acknowledged model-wait window, and added a controlled
dashboard workload. The dashboard polls every 500 ms, decodes 240 records,
filters and sorts them, formats values, reconciles 60 DOM rows, and persists a
small local state value. It contains no busy loop.

The matrix contained `normal-spa` and `agent-dashboard`, 2-second and 5-second
waits, concurrency one, the four primary mechanisms, and five randomized paired
blocks: 80 successful tasks in total.

At a 5-second wait on `agent-dashboard`, cgroup freeze reduced browser CPU from
0.9630 to 0.8877 seconds per successful task (7.8%) and net measured CPU from
1.6148 to 1.5259 seconds (5.5%). The paired mean net difference was -0.0889
seconds with a five-pair bootstrap 95% interval of [-0.1491, -0.0265]. The
diagnostic wait-window browser CPU fell from 0.1425 to 0.0255 seconds.

The corresponding 5-second `normal-spa` net reduction was 0.0690 seconds
(4.5%), with a paired bootstrap interval of [-0.0973, -0.0415]. At 2 seconds,
cgroup freeze produced no demonstrated complete-task reduction for either
workload even though wait-window CPU fell. This indicates a break-even effect:
the fixed cost and the first 500 ms left active consume most available savings
for short waits.

Chrome lifecycle freeze was neutral in this implementation. That result needs
further investigation because the CDP command is issued after the phase
acknowledgement and may not suppress the tested polling behavior as assumed.
The 25% CPU quota was also effectively neutral because baseline consumption was
already far below the quota.

These results are still not publication evidence. There are only five pairs,
only one VM lifetime, no concurrency result, and the controlled dashboard has
not been validated against a real application trace. More importantly, the
earlier gate showed cgroup freeze breaks the WebSocket oracle. The observed CPU
saving therefore describes a conditional tradeoff for sufficiently long,
correctly signaled waits on applications that tolerate full process suspension;
it does not justify enabling cgroup freeze by default.

Raw artifacts are retained locally under
`results/1787015970-agent-workload-gate-77dabfb1/`. They include the 80-task
event stream, environment record, processed summary, and chart.

## Long-wait correctness follow-up — 2026-08-18 UTC

A 48-task gate tested 5- and 10-second waits on `agent-dashboard` and the
WebSocket failure control with three randomized paired blocks. Cgroup freeze
reduced dashboard net CPU from 1.7543 to 1.5590 seconds at 10 seconds (11.1%),
while all six cgroup-freeze WebSocket tasks failed. The 5-second failures
produced three reconnects and 46 sequence gaps; the 10-second failures produced
three reconnects and 108 gaps. Baseline, lifecycle freeze, and CPU quota passed
all 18 corresponding WebSocket tasks.

The first dashboard background-operation counter in that run was invalid due
to a misplaced initializer. Its CPU and WebSocket results remain usable because
the counter was diagnostic and did not affect policy application, task oracles,
or resource accounting. The invalid dashboard counter is not interpreted.

A separate 24-task corrected diagnostic used authoritative server-side request
counts. Baseline, Chrome lifecycle freeze, and the 25% CPU quota each allowed
exactly 10 dashboard refreshes during a 5-second wait and 20 during a 10-second
wait. Cgroup freeze allowed a mean of 1.67 and 1.0 respectively. Thus the CDP
`Page.setWebLifecycleState("frozen")` call used here did not suspend this active
headless page's polling, while process-tree freeze did.

The result narrows Baeld's viable claim substantially. The only mechanism that
saved meaningful CPU in these controlled long waits was also the only mechanism
that repeatedly violated the live-session correctness oracle. Expanding to a
costly concurrency matrix would measure scaling of an unsafe default and is not
justified yet. The next engineering work should make compatibility explicit:
profile first, classify suspend-safe sessions, and keep cgroup freeze opt-in.

Raw artifacts are retained locally under
`results/1787017302-long-wait-correctness-gate-c8778ac1/` and
`results/1787017971-dashboard-freeze-diagnostic-c3dd78a8/`.

## Schema-4 concurrency validation — 2026-08-18 UTC

After adding explicit concurrency and randomized block identity, a bounded
Ubuntu 22.04 development gate ran two blocks at concurrency one and five. The
48 terminal tasks remained in distinct analyzer cells and paired by `block_id`,
validating the accounting repair that is required before any final matrix.

Cgroup freeze failed all 12 WebSocket tasks, with 12 reconnects and 39 sequence
gaps. The other mechanisms passed all 36 corresponding WebSocket tasks. The
normal-SPA CPU differences were small two-pair fluctuations and are not
performance evidence. This gate validates the harness, not a governor win.

Artifacts are retained locally under
`results/1787023423-concurrency-development-gate-dd0e305b/` and remain labeled
development-only because the host ran Ubuntu 22.04 and the gate had two pairs.

## Focused Ubuntu 24.04 pilot — 2026-08-18 UTC

The clean Ubuntu 24.04 host completed 480 task attempts across the controlled
dashboard and WebSocket workloads, 5- and 10-second model waits, concurrency
one and five, four mechanisms, and five randomized paired blocks. The harness
finished successfully after adding a bounded retry for transient `EBUSY` during
cgroup removal. The interrupted earlier attempt remains wholly excluded as
infrastructure-invalid.

All 120 dashboard tasks succeeded. Cgroup freeze reduced net measured CPU per
successful dashboard task by 0.061 seconds (3.7%) at 5 seconds/concurrency one,
0.074 seconds (3.5%) at 5 seconds/concurrency five, 0.135 seconds (7.9%) at 10
seconds/concurrency one, and 0.176 seconds (8.0%) at 10 seconds/concurrency
five. The respective five-pair bootstrap 95% intervals were [-0.071, -0.047],
[-0.106, -0.034], [-0.144, -0.123], and [-0.213, -0.148] seconds.

That saving did not survive the correctness constraint. Cgroup freeze failed
all 60 WebSocket tasks across every wait and concurrency cell. Baseline, Chrome
lifecycle freeze, and the 25% CPU quota passed all 180 of their corresponding
WebSocket tasks. Chrome lifecycle freeze and CPU quota produced mostly small,
directionally inconsistent net-CPU differences whose paired confidence
intervals crossed zero. The quota was not a useful optimization when default
browser consumption was already below it, and the tested Chrome lifecycle call
did not suppress the dashboard polling workload.

The honest pilot conclusion is negative for a general-purpose governor. Full
process-tree suspension can save a modest amount of CPU during sufficiently
long, explicitly signaled waits, but the same mechanism is unsafe for the
controlled live-connection workload. Baeld should profile first and expose
freeze only as an opt-in mechanism after workload-specific compatibility tests.
Five pairs on one VM lifetime are sufficient to justify this product-direction
decision, but not a broad production or population-level performance claim.

Raw events, the captured environment, aggregate summary, and paired analysis
are retained under
`results/1787025755-ubuntu24-focused-pilot-45a728b2/`. Cleanup validation found
no remaining Chromium process, workload server, or Baeld cgroup.

## Framework integration gates — 2026-08-19 UTC

Two clean 32-task gates tested Baeld-owned Chromium through Stagehand 4.0.1 and
Browser Use 0.13.6. Both ran from clean commit `a868eb4` on Ubuntu 24.04.4,
Chrome for Testing 151.0.7922.34, 8 dedicated vCPUs, and 32 GB RAM. Each gate
used eight randomized baseline/freeze pairs at a 5-second wait and concurrency
one for the controlled dashboard and WebSocket failure control.

All 32 dashboard tasks succeeded. Stagehand's frozen dashboard net CPU changed
by -0.0062 seconds per complete task (paired bootstrap 95% interval
[-0.0958, 0.0807]), so it demonstrated no complete-task saving. Browser Use's
point estimate was -0.0651 seconds, or 4.1% (interval [-0.1182, -0.0061]); its
browser-only interval still crossed zero. Both frameworks showed a clear
reduction inside the wait window, approximately 0.1800 and 0.1520 browser CPU
seconds respectively. The much smaller whole-task effects show that reporting
only wait-window CPU would overstate practical savings.

Correctness was categorical in this controlled workload. All 16 baseline
WebSocket tasks passed across the two frameworks, while all 16 cgroup-freeze
tasks failed. This reproduces the earlier Playwright result and makes a
framework-specific rescue of universal process suspension implausible.

These gates do not exercise Stagehand `act()` or Browser Use's model-backed
`Agent`; they measure deterministic framework attachment and session overhead.
Browser Use 0.13.6's element click helper did not dispatch the controlled
button event on this browser build, so the declared adapter uses its element
API for discovery and `Page.evaluate()` for the deterministic action. The
framework results support a measurement-first, compatibility-aware tool, not a
default freezing governor.

Raw artifacts are retained locally under
`results/1787110699-stagehand-deterministic-gate-f5b7d623/` and
`results/1787111322-browser-use-deterministic-gate-75022db4/`.
