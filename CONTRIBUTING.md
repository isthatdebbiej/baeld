# Contributing

The most valuable contribution is a reproducible workload where Baeld performs worse or produces a different correctness tradeoff.

Please include:

- workload classification and why it is representative;
- exact environment and pinned versions;
- authoritative success oracle;
- raw `events.jsonl` and `environment.json`;
- whether the result reproduces across multiple runs;
- no credentials, personal data, or unauthorized automation.

Before submitting code, run `just test`. Keep v0.1 changes narrowly tied to experimental validity.

