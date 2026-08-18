# Baeld

Baeld is an experimental Linux resource governor and reproducible benchmark for browser-agent inference waits.

It compares four mechanisms:

1. Default Chromium behavior.
2. Chromium-native page lifecycle freezing.
3. cgroup v2 CPU throttling.
4. cgroup v2 process-tree freezing.

Baeld asks which mechanism reduces **complete-task CPU per successful task** without hiding deferred work, correctness failures, WebSocket disruption, or governor overhead.

> v0.1.0-preview reports development evidence, not a production recommendation.
> Cgroup freeze reduced measured CPU for controlled 5–10 second waits, but
> repeatedly broke the WebSocket correctness oracle. Chrome lifecycle freeze
> and a 25% CPU quota were neutral in the tested setup. Baeld does not claim
> memory reclamation, increased browser density, or production readiness.

## Requirements

- Ubuntu 24.04 x86-64 or a comparable systemd-based Linux distribution.
- cgroup v2 with delegated `cpu`, `memory`, `io`, and `pids` controllers.
- Unprivileged Chromium sandboxing.
- Rust 1.85, Node 22, and pinned Playwright Chromium.

Windows is supported only for editing, JavaScript syntax checks, analysis, and Rust tests that do not require Linux.
The optional `scripts/dev-registry-proxy.mjs` exists only for restricted Windows development environments whose Schannel cannot reach crates.io; Linux CI and releases use crates.io directly.

## Compatibility

| Environment | Status | Intended use |
|---|---|---|
| Ubuntu 24.04 x86-64, cgroup v2 | Primary | Final reproducible measurements |
| Ubuntu 22.04 x86-64, cgroup v2 | Development-tested | Falsification and implementation gates |
| WSL2 | Development-only | Editing and short premise checks |
| Windows and macOS hosts | No benchmark support | Editing, pure tests, and analysis only |
| Root or `--no-sandbox` Chromium | Refused | Unsupported and unsafe |

## Setup

```bash
git clone https://github.com/isthatdebbiej/baeld.git
cd baeld
scripts/setup-ubuntu.sh
bash scripts/run-cloud-gate.sh
```

Baeld refuses publishable benchmarks if required host controls or Chromium sandboxing are unavailable.

## Fast loop

```bash
just test     # pure tests and JavaScript syntax
just smoke    # short four-mechanism experiment
just pilot    # controlled pilot matrix
```

### WSL2 development gate

WSL2 is supported only for falsifying the premise and debugging Baeld. Its results
must be labeled as development results and must not be used for headline claims.
Keep the repository in WSL's Linux filesystem, close memory-heavy Windows programs,
and require at least 3 GiB of available memory before starting:

```bash
scripts/check-wsl.sh
cargo build --release --locked
scripts/run-scoped.sh doctor
scripts/run-scoped.sh bench --config experiments/wsl-gate.toml
```

The WSL gate is intentionally restricted to concurrency one, the representative SPA
and WebSocket failure control, two wait durations, and the four primary mechanisms.

For a disposable GCP or other Ubuntu VM, follow the
[cloud sandbox runbook](docs/cloud-sandbox.md). It separates cheap development and
Spot runs from the on-demand host used for final measurements.

Summarize and plot a run:

```bash
cargo run -- summarize results/<run-id>
just plot results/<run-id>
python analysis/paired.py results/<run-id>
```

`smoke` exits successfully when the harness and cleanup complete, even when a
mechanism fails an application oracle. Those failures are benchmark evidence
and remain in `events.jsonl`; infrastructure errors still make the command
fail.

`summarize` reports net CPU change against the matching baseline and labels
compatibility evidence as `failure-observed` or `no-failure-observed`. The
latter deliberately does not mean safe. Cells with fewer than the declared 20
runs remain `development-only` in JSON output.
Event schema 4 records concurrency and randomized block identity on every
terminal task. The paired analyzer keeps concurrency cells separate, pairs by
block identity, and retains failed-attempt CPU in the correctness-adjusted
numerator. Older schema 1–3 datasets remain readable, but occurrence-order
pairing is allowed only for their concurrency-one runs.

## Correctness model

The static workload checks an exact DOM oracle. Mutating workloads verify a server-persisted update occurred exactly once. The controlled agent dashboard performs ordinary polling, JSON processing, sorting, formatting, and DOM reconciliation without a synthetic busy loop. The WebSocket workload reports reconnects and missing sequence numbers. Every measured task includes a post-resume settling window. Baeld reports complete-task CPU as the primary metric and browser CPU during the model-wait window as a diagnostic explaining where changes occur.

## Security

Baeld never requires Chromium to run as root and never passes `--no-sandbox`. The setup grants a narrow cgroup subtree to the experiment user. Run only controlled or explicitly authorized web workloads.

## Project status

The current objective is to falsify or validate the underlying hypothesis before expanding the CLI. See [methodology](report/methodology.md), [limitations](report/limitations.md), and the [experiment ledger](experiments/LOG.md).

## Release packaging

From a clean exact release commit, create the same Linux artifact as CI with:

```bash
scripts/package-release.sh v0.1.0
```

Pushing a `v*` tag creates a GitHub release containing the Linux x86-64 archive
and SHA-256 checksum. Raw datasets remain a deliberate manual attachment; the
workflow never uploads ignored development results.

## License

Licensed under either the Apache License, Version 2.0 or the MIT License at your option.
