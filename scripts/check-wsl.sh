#!/usr/bin/env bash
set -euo pipefail

fail=0

check() {
  local name="$1"
  shift
  if "$@" >/dev/null 2>&1; then
    printf 'PASS  %s\n' "$name"
  else
    printf 'FAIL  %s\n' "$name"
    fail=1
  fi
}

printf 'Baeld WSL development preflight\n\n'
check "WSL2 kernel" grep -qi microsoft /proc/sys/kernel/osrelease
check "systemd is PID 1" sh -c '[ "$(ps -p 1 -o comm= | xargs)" = systemd ]'
check "cgroup v2" sh -c '[ "$(stat -fc %T /sys/fs/cgroup)" = cgroup2fs ]'
check "CPU controller" grep -qw cpu /sys/fs/cgroup/cgroup.controllers
check "memory controller" grep -qw memory /sys/fs/cgroup/cgroup.controllers
check "I/O controller" grep -qw io /sys/fs/cgroup/cgroup.controllers
check "CPU PSI" test -r /proc/pressure/cpu
check "memory PSI" test -r /proc/pressure/memory
check "at least 3 GiB available" sh -c '[ "$(awk "/MemAvailable/ {print \$2}" /proc/meminfo)" -ge 3145728 ]'
check "pinned Chromium" test -x .baeld/chromium

printf '\n'
if [[ "$fail" -ne 0 ]]; then
  printf 'WSL is not ready. Do not benchmark until the failed checks are corrected.\n' >&2
  exit 1
fi

printf 'WSL is suitable for the development gate, not headline measurements.\n'
