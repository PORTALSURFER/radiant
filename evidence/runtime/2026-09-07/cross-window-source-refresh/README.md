# cross-window-source-refresh — OPT-1363 partial delivery

Source: `06674c45846b7abbf67343cfe75f19d9e28c63bf`.

Receiver reductions refresh and requalify auxiliary sources before further mapping. Missing source projections stop drag capture and new input admission while preserving admitted-ticket completion. Nonconvergent refreshes and stale terminal fallback fail closed. Four new headless regressions pass; this bundle supersedes earlier source validation for PR1904. All native and manual acceptance remains NOT_RUN.

C/PASS and A/PASS cover completed local static/core assertions only. H/NOT_RUN and M/NOT_RUN: no native or manual acceptance was attempted. This candidate must merge before shipped claims; no three-platform acceptance is inferred.

Counts: {"tests": {"passed": 5636, "ignored": 8}, "examples": {"passed": 296, "ignored": 0}, "doctests": {"passed": 20, "ignored": 1}}. Strict Clippy, warning-denied docs, host no-default-features, formatting, and diff checks passed. Complete command outcomes are in validation.json. Any compiler/linker warnings remain visible in the logs. Remaining ticket scope is tracked in Linear OPT-1363.
