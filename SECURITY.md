# Security policy

Baeld is an experimental research tool, not a security boundary. It controls complete Chromium process trees and should run only on isolated Linux workers with workloads you are authorized to automate.

Do not disable Chromium's sandbox, run Chromium as root, place multiple tenants in one delegated cgroup subtree, or include secrets in released benchmark artifacts.

Persistent browser profiles contain cookies and authentication state. Give each identity a dedicated profile path and filesystem owner; never reuse a profile concurrently or attach it to an untrusted agent. JSONL and OTLP events exclude page content and command arguments by design, but executable paths, workload names, and error messages may still be sensitive.

The CDP listener binds to localhost and the phase socket is created in the user's state directory. Do not expose either endpoint through a public proxy. Baeld resource limits reduce accidental pressure; they are not a tenant isolation or authorization boundary.

On Ubuntu 23.10 and newer, `scripts/setup-ubuntu.sh` installs a root-owned
AppArmor profile granting `userns` only to the exact pinned Playwright Chromium
path. It does not disable `kernel.apparmor_restrict_unprivileged_userns`
globally. Re-run setup when the pinned Playwright browser path changes.

Report vulnerabilities privately through GitHub security advisories after the repository is published.
