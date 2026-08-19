# Troubleshooting

- **Missing delegation:** run through `scripts/run-scoped.sh`; never work around it by running Chromium as root.
- **Child cannot connect:** use an adapter inside `baeld run` and check the three `BAELD_*` connection variables.
- **Rejected phase:** check ordering, generation, and the required expected duration for model waits.
- **Filtering breaks a site:** switch to `safe` or `visual` and re-run the authoritative oracle.
- **Live connection reconnects:** stop using freeze, mark the connection critical, and use `safe` or compatibility-required `adaptive`.
- **Stale metadata:** inspect with `baeld sessions`, then run `baeld cleanup`; live runtime records are preserved.
