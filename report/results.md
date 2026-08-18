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
