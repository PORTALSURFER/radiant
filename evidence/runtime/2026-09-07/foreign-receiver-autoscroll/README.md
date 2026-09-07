# foreign-receiver-autoscroll — OPT-1363 partial delivery

Source: `2eb4ae3b4f91fdb042bad94f7e851691c9dd0545`.

Foreign receivers scroll only after successful timed-drain completion and native/source/receiver requalification, with scroll reductions before target reentry. Failed deadlines and retired sources cannot rearm or replay. Fresh package artifacts were rebuilt after rejecting an earlier stale-cache run; all eight new autoscroll regressions were observed passing. C/A are local only; native/manual acceptance remains NOT_RUN.

C/PASS and A/PASS cover completed local static/core assertions only. H/NOT_RUN and M/NOT_RUN: no native or manual acceptance was attempted. This candidate must merge before shipped claims; no three-platform acceptance is inferred.

Counts: {"tests": {"passed": 5644, "ignored": 8}, "examples": {"passed": 296, "ignored": 0}, "doctests": {"passed": 20, "ignored": 1}}. Strict Clippy, warning-denied docs, host no-default-features, formatting, and diff checks passed. Complete command outcomes are in validation.json. Any compiler/linker warnings remain visible in the logs. Remaining ticket scope is tracked in Linear OPT-1363.
