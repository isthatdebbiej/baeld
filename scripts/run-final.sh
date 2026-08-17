#!/usr/bin/env bash
set -euo pipefail

[[ -z "$(git status --porcelain)" ]] || {
  echo "Refusing final benchmark from a dirty worktree" >&2
  exit 1
}

scripts/verify-host.sh
mkdir -p results
cargo build --release --locked
BAELD_CONTROLLER_CPUS=0-1 \
  scripts/run-scoped.sh bench --config experiments/final.toml --output results
