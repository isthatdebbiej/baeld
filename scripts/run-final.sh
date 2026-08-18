#!/usr/bin/env bash
set -euo pipefail

[[ -z "$(git status --porcelain)" ]] || {
  echo "Refusing final benchmark from a dirty worktree" >&2
  exit 1
}

scripts/verify-host.sh

cpus="$(nproc)"
memory_kib="$(awk '/MemTotal:/ {print $2}' /proc/meminfo)"
if (( cpus < 8 )); then
  echo "Final benchmark requires at least 8 logical CPUs for affinity 0-1 and 2-7; found $cpus" >&2
  exit 1
fi
if (( memory_kib < 31457280 )); then
  echo "Final benchmark requires at least 30 GiB visible RAM; found $((memory_kib / 1024)) MiB" >&2
  exit 1
fi

mkdir -p results
cargo build --release --locked
BAELD_CONTROLLER_CPUS=0-1 \
  scripts/run-scoped.sh bench --config experiments/final.toml --output results
