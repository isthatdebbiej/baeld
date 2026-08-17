#!/usr/bin/env bash
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
cd "$root"

scripts/verify-host.sh

cpus="$(nproc)"
memory_kib="$(awk '/MemTotal:/ {print $2}' /proc/meminfo)"
if (( cpus < 4 )); then
  echo "Cloud gate requires at least 4 logical CPUs; found $cpus" >&2
  exit 1
fi
if (( memory_kib < 7864320 )); then
  echo "Cloud gate requires at least 8 GiB RAM; found $((memory_kib / 1024)) MiB" >&2
  exit 1
fi

cargo build --release --locked

echo "==> Host diagnostics"
scripts/run-scoped.sh doctor

echo "==> Four-mechanism smoke test"
scripts/run-scoped.sh smoke

echo "==> Paired cloud development gate"
scripts/run-scoped.sh bench --config experiments/cloud-gate.toml

echo "Cloud gate complete. These are development results, not headline measurements."
