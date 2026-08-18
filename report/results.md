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
