#!/usr/bin/env bash
set -euo pipefail

[[ -z "$(git status --porcelain)" ]] || {
  echo "Refusing final benchmark from a dirty worktree" >&2
  exit 1
}

scripts/verify-host.sh
mkdir -p results
cargo build --release --locked
taskset -c 0-1 target/release/baeld bench --config experiments/final.toml --output results
