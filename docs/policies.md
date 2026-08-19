# Policy and safety guide

Baeld restores unrestricted CPU and thaws the browser before acknowledging any non-waiting phase. Delayed freezes carry the phase generation and cannot freeze a newer action.

Use `observe` to establish normal behavior. Use `safe` for limits, cleanup, and scheduling without suspension. Use `adaptive` after adding accurate phases; it leaves short waits and critical live connections unrestricted and throttles unknown long waits. Use `explicit` for controlled comparisons or validated workloads.

`freeze_compatibility = "required"` prevents adaptive full freeze. `confirmed` is an operator assertion after the workload passes authentication expiry, WebSocket/SSE, polling, worker, timer, resume, verification, and settling tests.

Invalid phases are rejected. Policy errors trigger thaw and unrestricted CPU. Acting is acknowledged only after restoration and admission. If the controller dies, Baeld terminates the owned Chromium tree and preserves a session record.
