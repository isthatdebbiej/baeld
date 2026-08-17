#!/usr/bin/env bash
set -euo pipefail

test "$(stat -fc %T /sys/fs/cgroup)" = "cgroup2fs"
test -r /proc/pressure/cpu
test -r /proc/pressure/memory
test -r /proc/pressure/io
test -x .baeld/chromium
test "$(ps -p 1 -o comm= | xargs)" = "systemd"
command -v systemd-run >/dev/null
command -v taskset >/dev/null
