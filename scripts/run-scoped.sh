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

current_cgroup="$(awk -F: '$1 == 0 { print $3 }' /proc/self/cgroup)"
controllers="$(cat "/sys/fs/cgroup${current_cgroup}/cgroup.controllers")"
for required in cpu io memory pids; do
  if [[ " $controllers " != *" $required "* ]]; then
    if ! sudo -n true 2>/dev/null; then
      echo "User scope lacks the $required controller and passwordless sudo is unavailable" >&2
      exit 1
    fi
    # Older systemd releases commonly delegate only memory and pids to user
    # scopes. A transient system service lets PID 1 delegate all controllers
    # while applying the invoking user's complete credentials and HOME before
    # Baeld or Chromium starts.
    exec sudo systemd-run \
      --wait \
      --collect \
      --pipe \
      --quiet \
      -p Delegate=yes \
      --uid="$(id -un)" \
      --working-directory="$root" \
      "${command[@]}"
  fi
done

# Run Baeld directly in the delegated scope. An interactive shell or `cargo run`
# would remain in the cgroup root and violate cgroup v2's no-internal-process rule.
exec systemd-run \
  --user \
  --scope \
  --collect \
  --quiet \
  -p Delegate=yes \
  "${command[@]}"
