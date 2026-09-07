# outgoing-single-url-drag — OPT-1363 partial delivery

Source: `faabad6bc8820361fdc57f761fb0fb47c5cb4b16`.

One deliberately exported URL uses an NSURL pasteboard writer on macOS and UniformResourceLocatorW on Windows. Shared launch-time validation bounds and checks direct-enum and constructor requests before native context access. A fresh package build from this exact worktree passed all checks and the new URL regressions explicitly executed. Native/manual drag acceptance was not attempted.

C/PASS and A/PASS cover completed local static/core assertions only. H/NOT_RUN and M/NOT_RUN: no native or manual acceptance was attempted. This candidate must merge before shipped claims; no three-platform acceptance is inferred.

Counts: {"tests": {"passed": 5627, "ignored": 8}, "examples": {"passed": 296, "ignored": 0}, "doctests": {"passed": 20, "ignored": 1}}. Strict Clippy, warning-denied docs, host no-default-features, formatting, and diff checks passed. Complete command outcomes are in validation.json. Any compiler/linker warnings remain visible in the logs. Remaining ticket scope is tracked in Linear OPT-1363.
