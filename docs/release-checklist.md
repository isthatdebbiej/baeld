# Release checklist

- Pin and record Rust, Bun, Node, Python, Chromium, Playwright, Stagehand, and Browser Use.
- Pass formatting, Clippy, unit, adapter, packaging, and clean Ubuntu integration jobs.
- Run deterministic and provider-configured real-agent gates for all supported frameworks.
- Run 100- and 500-task lifecycle gates with controller-crash injection.
- Confirm no browser descendants, profiles, sockets, permits, or cgroups remain.
- Regenerate charts from attached raw data and list every exclusion.
- Verify README commands from clean installations of the binary, npm package, and Python wheel.
- Keep WebSocket freeze failures and framework limitations visible.
- Generate archive checksums, SBOM, changelog, and build provenance.
- Tag only the exact tested commit.
