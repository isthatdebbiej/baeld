#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -eq 0 ]]; then
  echo "usage: scripts/run-scoped.sh <baeld arguments...>" >&2
  exit 2
fi

root="$(git rev-parse --show-toplevel)"
binary="$root/target/release/baeld"

[[ -x "$binary" ]] || {
  echo "Missing $binary; run cargo build --release --locked" >&2
  exit 1
}

command=("$binary" "$@")
if [[ -n "${BAELD_CONTROLLER_CPUS:-}" ]]; then
  command=(taskset -c "$BAELD_CONTROLLER_CPUS" "$binary" "$@")
fi

# Run Baeld directly in the delegated scope. An interactive shell or `cargo run`
# would remain in the cgroup root and violate cgroup v2's no-internal-process rule.
exec systemd-run \
  --user \
  --scope \
  --collect \
  --quiet \
  -p Delegate=yes \
  "${command[@]}"
