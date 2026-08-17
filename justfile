set shell := ["bash", "-cu"]

test:
    cargo test
    npm run check

doctor:
    cargo run -- doctor

smoke:
    cargo run -- smoke --output results

wsl-check:
    scripts/check-wsl.sh

wsl-gate:
    cargo run --release -- bench --config experiments/wsl-gate.toml --output results

pilot:
    cargo run --release -- bench --config experiments/pilot.toml --output results

full:
    scripts/run-final.sh

plot run:
    .venv/bin/python analysis/analyze.py "{{run}}"
