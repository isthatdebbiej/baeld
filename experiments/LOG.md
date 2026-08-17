# Experiment ledger

Every experiment entry must include its hypothesis, exact command, result, decision, and artifact directory. Results are never overwritten.

## E000 — Unrun baseline

- Hypothesis: At least one suspension mechanism changes complete-task CPU or correctness relative to default Chromium.
- Command: `just smoke`
- Result: Not run; requires the delegated Ubuntu environment.
- Decision: Run before adding any optional feature.
- Artifacts: Pending.

