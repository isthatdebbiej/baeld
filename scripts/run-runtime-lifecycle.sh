#!/usr/bin/env bash
set -euo pipefail

config="${BAELD_CONFIG:-baeld.toml}"

scripts/run-scoped.sh run --config "$config" -- node workloads/driver/runtime-fixture.mjs normal
if scripts/run-scoped.sh run --config "$config" -- node workloads/driver/runtime-fixture.mjs exit; then
  echo "expected injected non-zero exit" >&2
  exit 1
fi
if scripts/run-scoped.sh run --config "$config" -- node workloads/driver/runtime-fixture.mjs sigkill; then
  echo "expected injected SIGKILL" >&2
  exit 1
fi
target/release/baeld cleanup

if pgrep -af 'baeld-chrome-' >/dev/null; then
  echo "Chromium fixture process survived runtime cleanup" >&2
  pgrep -af 'baeld-chrome-' >&2
  exit 1
fi
