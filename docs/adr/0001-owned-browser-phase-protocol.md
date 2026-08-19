# ADR 0001: Own Chromium and require explicit phases

Status: accepted.

Baeld launches Chromium into its cgroup before execution and exposes a localhost CDP endpoint. Framework adapters emit acknowledged phases over a versioned Unix socket. Arbitrary process discovery was rejected because it cannot reliably distinguish the agent driver from browser descendants or restore resources before an action.

This decision makes framework cooperation necessary for phase-aware policies, while uninstrumented commands still receive monitoring and cleanup.
