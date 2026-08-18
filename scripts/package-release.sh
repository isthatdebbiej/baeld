#!/usr/bin/env bash
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
cd "$root"

version="${1:-$(git describe --tags --exact-match 2>/dev/null || true)}"
if [[ ! "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.-]+)?$ ]]; then
  echo "Usage: $0 v<semver> (or run from an exact v<semver> tag)" >&2
  exit 2
fi

[[ -z "$(git status --porcelain)" ]] || {
  echo "Refusing to package a dirty worktree" >&2
  exit 1
}

cargo test --locked
cargo build --release --locked

package="baeld-${version}-linux-x86_64"
stage="dist/$package"
rm -rf -- "$stage"
mkdir -p "$stage/report"
install -m 755 target/release/baeld "$stage/baeld"
install -m 644 README.md CHANGELOG.md LICENSE-MIT LICENSE-APACHE "$stage/"
install -m 644 report/methodology.md report/limitations.md report/results.md "$stage/report/"
tar -C dist -czf "dist/$package.tar.gz" "$package"
sha256sum "dist/$package.tar.gz" > "dist/$package.tar.gz.sha256"
printf 'Created %s\n' "dist/$package.tar.gz"
