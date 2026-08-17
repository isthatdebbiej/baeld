#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "Baeld setup requires Linux" >&2
  exit 1
fi

sudo apt-get update
sudo apt-get install -y build-essential curl git jq just nodejs npm pkg-config python3 python3-venv

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

python3 -m venv .venv
.venv/bin/pip install -r analysis/requirements.txt

echo
echo "Setup complete. Enter a delegated scope before benchmarking:"
echo "  systemd-run --user --scope -p Delegate=yes --collect bash"
echo "Then run: cargo run -- doctor"
