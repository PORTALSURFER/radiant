# external-offer-eligibility-probe — OPT-1363 partial delivery

Source: `7d194f86e1738f1a6f2bc79bb21a20e96651943d`.

A side-effect-free external-offer eligibility probe shares the existing bounded current-target selector with final drop admission. Full validation is at source7d194f86; new capacity and owner/modal/geometry regressions execute explicitly. Initial fixture compile errors and failed width assumptions were corrected before this run: the target is a child with explicit Start cross alignment and pre/post bounds assertions. This final retry used the same worktree and unchanged production library, with no intervening Cargo worktree switch; the updated integration binary was recompiled from this exact worktree. No native extraction, window interaction or manual acceptance was attempted.

C/PASS and A/PASS cover completed local static/core assertions only. H/NOT_RUN and M/NOT_RUN: no native or manual acceptance was attempted. This candidate must merge before shipped claims; no three-platform acceptance is inferred.

Counts: {"tests": {"passed": 5638, "ignored": 8}, "examples": {"passed": 296, "ignored": 0}, "doctests": {"passed": 20, "ignored": 1}}. Strict Clippy, warning-denied docs, host no-default-features, formatting, and diff checks passed. Complete command outcomes are in validation.json. Any compiler/linker warnings remain visible in the logs. Remaining ticket scope is tracked in Linear OPT-1363.
