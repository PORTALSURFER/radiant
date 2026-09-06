# Resource state and declarative views (OPT-1391)

Final source: `b8904065bf6ed217ecc04c55e9484ea71c9d3b05`. The full library suite ran on production commit `0cbb8808`; later changes replaced guarded expects with fail-closed handling, removed an unsafe default resource key, and fixed ownership in the documentation example. Integration, examples and fixture ran after the production guard fixes (`483629d90ab7eaba0e6ee798aea2977359e3fd5f`); documentation and quality checks ran with the final source tree.

Resource values and errors remain application-owned. Operation snapshots fence completion, progress, retry and cancellation. Bounded predecessor state restores rejected refreshes, including retained ready values. ResourceView selects ordinary view branches and contributes bounded consumer demand only after projection acceptance. Removal and shutdown retire those interests.

The deterministic headless fixture proves shared-consumer ready delivery, retained-value failure, cancellation rejecting a late replacement, and final interest release. Unit and public tests cover retargeting, rejected admission rollback, independent resource identities, retry deadlines, progress ordering, source demand overflow, key changes and shutdown.

Validation: full library suite 4302 passed / 8 existing ignored; all integration and example tests passed; doctests, documentation build, strict all-target/all-feature Clippy, no-default library check, formatting and diff checks passed. See raw logs for counts and exact output. No foreground native or performance acceptance is claimed.
