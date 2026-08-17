#!/usr/bin/env bash
set -euo pipefail

test "$(stat -fc %T /sys/fs/cgroup)" = "cgroup2fs"
test -r /proc/pressure/cpu
test -r /proc/pressure/memory
test -r /proc/pressure/io
test -x .baeld/chromium

current="$(awk -F: '$1=="0" {print $3}' /proc/self/cgroup)"
root="/sys/fs/cgroup${current}"
test -w "$root/cgroup.subtree_control" || {
  echo "Current scope is not delegated: $root" >&2
  echo "Run inside: systemd-run --user --scope -p Delegate=yes --collect bash" >&2
  exit 1
}

cargo run -- doctor
