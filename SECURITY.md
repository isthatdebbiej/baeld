# Security policy

Baeld is an experimental research tool, not a security boundary. It controls complete Chromium process trees and should run only on isolated Linux workers with workloads you are authorized to automate.

Do not disable Chromium's sandbox, run Chromium as root, place multiple tenants in one delegated cgroup subtree, or include secrets in released benchmark artifacts.

On Ubuntu 23.10 and newer, `scripts/setup-ubuntu.sh` installs a root-owned
AppArmor profile granting `userns` only to the exact pinned Playwright Chromium
path. It does not disable `kernel.apparmor_restrict_unprivileged_userns`
globally. Re-run setup when the pinned Playwright browser path changes.

Report vulnerabilities privately through GitHub security advisories after the repository is published.
