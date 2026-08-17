# Security policy

Baeld is an experimental research tool, not a security boundary. It controls complete Chromium process trees and should run only on isolated Linux workers with workloads you are authorized to automate.

Do not disable Chromium's sandbox, run Chromium as root, place multiple tenants in one delegated cgroup subtree, or include secrets in released benchmark artifacts.

Report vulnerabilities privately through GitHub security advisories after the repository is published.

