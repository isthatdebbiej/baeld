#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "Baeld setup requires Linux" >&2
  exit 1
fi

sudo apt-get update
sudo apt-get install -y build-essential ca-certificates curl git jq pkg-config python3 python3-venv

node_major="$(node -p 'process.versions.node.split(".")[0]' 2>/dev/null || echo 0)"
if (( node_major < 22 )); then
  curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
  sudo apt-get install -y nodejs
fi

if ! command -v rustup >/dev/null; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
fi
source "$HOME/.cargo/env"
rustup toolchain install 1.85.0 --profile minimal
rustup override set 1.85.0

npm ci
npx playwright install --with-deps chromium
mkdir -p .baeld
chrome_path="$(node -e "import('playwright').then(p => console.log(p.chromium.executablePath()))")"
ln -sfn "$chrome_path" .baeld/chromium

# Ubuntu 23.10+ restricts unprivileged user namespaces through AppArmor. A
# downloaded Chrome-for-Testing binary consequently cannot start its sandbox
# unless its exact path is allowlisted. Keep the global restriction enabled and
# install the narrow profile recommended by Chromium rather than using
# --no-sandbox or disabling the sysctl system-wide.
if [[ -r /proc/sys/kernel/apparmor_restrict_unprivileged_userns ]] \
  && [[ "$(cat /proc/sys/kernel/apparmor_restrict_unprivileged_userns)" == "1" ]]; then
  case "$chrome_path" in
    "$HOME"/.cache/ms-playwright/*/chrome-linux64/chrome) ;;
    *)
      echo "Refusing to allowlist unexpected Chromium path: $chrome_path" >&2
      exit 1
      ;;
  esac
  sudo tee /etc/apparmor.d/baeld-playwright-chromium >/dev/null <<EOF
abi <abi/4.0>,
include <tunables/global>

profile baeld-playwright-chromium $chrome_path flags=(unconfined) {
  userns,
}
EOF
  sudo apparmor_parser -r /etc/apparmor.d/baeld-playwright-chromium
fi

python3 -m venv .venv
.venv/bin/pip install -r analysis/requirements.txt
cargo build --release --locked

echo
echo "Setup complete. Run the disposable cloud gate with:"
echo "  bash scripts/run-cloud-gate.sh"
