# Shared resource lifecycle validation

Production implementation: `704129e57eca2d0c5dca9222626c2e8be5e133b9`.
Final source: `866d0e493713b37fc7019a1fecaf34402c30776c`. Follow-ups register the integration suite, regroup exports, format tests, replace `or_insert_with(Default)` with `or_default`, and name a test helper type.

- Library suite: 4,275 passed, 8 existing ignores, on the production implementation.
- Integration suite: 1,088 passed, including 315 guardrails and 8 public resource tests.
- Examples: 273 passed. Doctests: 19 passed, 1 existing ignore. Documentation built.
- Strict all-target/all-feature Clippy, no-default-feature library check, formatting, and diff checks passed on final source.
- The deterministic lifecycle fixture emitted `lifecycle.jsonl`: two interests share one worker; starter release preserves delivery; eligible ready state is reused; final release suppresses a late refresh. It opens no native window and performs no provider IO.

Integration/example/doc validation uses the tree recorded by `526d7343331f6de921e7ba3430c0d4fe109e49cf`; only the equivalent default insertion spelling and test type alias changed afterward. Failed intermediate runs are preserved in local validation logs, not presented as passing evidence.

Independent review covered interest leases and generation retirement, runtime capacity/binding, operation admission and rollback, token/key cancellation, retry, retention, and shutdown. Reported cancellation defects were fixed with targeted regressions before the passing library run.

This bundle establishes deterministic lifecycle correctness, not a throughput, foreground latency, or three-platform host acceptance claim. OPT-1388 performance acceptance remains separate.
