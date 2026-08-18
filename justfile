set shell := ["bash", "-cu"]

test:
    cargo test
    bun run check

doctor:
    cargo build --release --locked
    scripts/run-scoped.sh doctor

smoke:
    cargo build --release --locked
    scripts/run-scoped.sh smoke --output results

wsl-check:
    scripts/check-wsl.sh

wsl-gate:
    cargo build --release --locked
    scripts/run-scoped.sh bench --config experiments/wsl-gate.toml --output results

cloud-gate:
    scripts/run-cloud-gate.sh

pilot:
    cargo build --release --locked
    scripts/run-scoped.sh bench --config experiments/pilot.toml --output results

full:
    scripts/run-final.sh

plot run:
    .venv/bin/python analysis/analyze.py "{{run}}"
