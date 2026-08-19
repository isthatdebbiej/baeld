# Reproducing benchmarks

Use a clean Ubuntu 24.04 x86-64 host with pinned dependencies and no unrelated workload.

```bash
scripts/setup-ubuntu.sh
scripts/run-scoped.sh doctor
scripts/run-scoped.sh smoke --output results
scripts/run-scoped.sh bench --config experiments/ubuntu24-focused-pilot.toml --output results
cargo run -- summarize results/<run-id>
python analysis/analyze.py results/<run-id>
python analysis/paired.py results/<run-id>
```

Application failures remain in the dataset. Exclude only infrastructure-invalid runs under predeclared rules. Publish environment, raw events, processed output, configuration, commit, and exclusion reasons.

The WebSocket workload is a failure control. A policy that reduces activity while violating its oracle is incompatible, not successful.
