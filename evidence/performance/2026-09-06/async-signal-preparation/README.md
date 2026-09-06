# Asynchronous raw signal preparation validation

Final production revision: `514a2fef6d754ec2d04010d39e1ab670b18da4ca`.

At this revision: 47 focused summary tests, one real native async signal retirement test, 315 architecture guardrails, strict all-target/all-feature Clippy and formatting passed. The deterministic final-drop test runs maintenance while the off-thread wake is still executing, verifying that the cache token has already been released.

Before that token-order-only fix, integrated revision `3b2292eac0414b943218c4b5fe36641a171fcd25` passed 223 GPU tests and all six native GPU tests (five shared-shader transaction tests plus async signal retirement). These raw logs are retained separately; they are not claimed as reruns of the final revision.

Commands used `CARGO_INCREMENTAL=0`, the shared validation target directory and `--locked --all-features`. Focused library filters were `signal_summary`, `gpu_surface` and `async_signal_native_tests`; native tests added `-- --ignored --test-threads=1`. Guardrails used `--test generic_surface_guardrails`; Clippy used `--all-targets --all-features -- -D warnings`; formatting used `cargo fmt --all -- --check`.

## Acceptance still pending

These checks establish lifecycle, cache ownership and actual offscreen rendering behavior. They do not establish foreground input responsiveness or cold long-source latency improvement. OPT-1456 remains open for that native baseline/candidate comparison.

The existing 600-second stereo recorder source requires approximately 1.152 GB of logical raw-source plus full-pyramid ownership, exceeding the explicit 256 MiB preparation budget. It is an unavailable-capacity case, not an accepted worker workload. A separate 60-second stereo input fixture fits the budget (approximately 115.2 MB). Neither workload has a foreground timing result in this evidence pack.

Worker preparation time, reserved logical bytes and retained ready-summary bytes are separate diagnostics. Logical ownership is not RSS or transient allocation peak.
