# Contributing

The most valuable contribution is a reproducible workload where Baeld helps, is neutral, performs worse, or produces a different correctness tradeoff. Start with an issue before changing a public protocol or policy default.

Please include:

- workload classification and why it is representative;
- exact environment and pinned versions;
- authoritative success oracle;
- raw `events.jsonl` and `environment.json`;
- whether the result reproduces across multiple runs;
- no credentials, personal data, or unauthorized automation.

## Development setup

Use Rust 1.85, Bun 1.3.14, Node 22.18, and Python 3.12. On any host:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
bun install --frozen-lockfile
bun run check
PYTHONPATH=packages/python/src python -m unittest discover packages/python/tests -v
```

Runtime changes also require the clean Ubuntu integration workflow. Verify browser membership before each measured block and confirm cleanup after driver crashes. Do not loosen Chromium sandboxing to make a test pass.

Keep public documentation written for users. Put durable design decisions in `docs/adr`; put reproducibility details in the experiment ledger. Never omit application failures from benchmark data.
