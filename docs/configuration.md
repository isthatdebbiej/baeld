# Configuration reference

Baeld reads versioned TOML. Start with [`baeld.example.toml`](../baeld.example.toml); unknown keys and unsupported schemas are rejected.

| Key | Meaning |
|---|---|
| `schema_version` | v0.1 accepts `1`. |
| `chromium` | Pinned local Chromium executable. |
| `chrome_args` | Additional flags; security-disabling flags are unsupported. |
| `allow_extensions` | Enables extensions for integrations such as Stagehand local attachment. |

## Policy

`policy.mode` accepts `observe`, `safe`, `adaptive`, or `explicit`. `sample_ms` controls health sampling and cannot be below 100 ms. `phase_timeout_ms` marks an unresponsive instrumented session stuck.

```toml
[policy]
mode = "explicit"

[policy.explicit]
mechanism = "cgroup-freeze"
delay_ms = 500
```

For CPU quota, use `mechanism = "cpu-quota"`, `quota_us`, and `period_us`.

Adaptive freeze requires `freeze_compatibility = "confirmed"` and a `compatibility_file`. Records match the exact framework, workload, browser version, and mechanism; a global “safe” flag is not accepted.

## Session

- `ephemeral` creates and removes a temporary browser profile.
- `persistent` requires `profile_dir` and preserves identity state for that session owner.
- `warm` keeps one browser available for the supervised command while adapters create fresh contexts.

`max_memory_mb` maps to `memory.max` with an earlier `memory.high` signal; `max_processes` maps to `pids.max`. Set `recycle_on_degraded = true` only when an external supervisor will restart terminated work; it is disabled by default. Baeld never shares persistent profiles between sessions.

## Filtering

- `safe`: no filtering.
- `visual`: preserves visual behavior.
- `balanced`: blocks media, common tracker hosts, and animations through the adapter.
- `text`: additionally blocks images and fonts and disables GPU rendering.

Filtering changes page behavior. Validate the authoritative application result before broad use.

## Concurrency and telemetry

Navigation and action limits are machine-wide cooperative permits. New expensive phases wait while host CPU or memory PSI exceeds the configured `avg10` thresholds. `stagger_resume_ms` serializes resume admission. JSONL is local; a `unix:///path/to/collector.sock` `otel_endpoint` receives OTLP/HTTP JSON at `/v1/logs`. The local collector owns authenticated or TLS onward export.
