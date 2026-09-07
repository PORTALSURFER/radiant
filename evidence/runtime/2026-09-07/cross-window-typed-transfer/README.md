# cross-window-typed-transfer — OPT-1363 partial delivery

Source: `6b25e7dc397707282f0964a548e3fe21c7dacbd6`.

Same-application typed drags preserve source capture and payload identity across primary and auxiliary macOS/Windows windows. Semantic callbacks remain inside the original native input ticket; terminal target/source messages are mapped before either reducer, with visual-only deferral and qualified retirement cleanup. Headless regressions cover cross-window ordering, cancellation, replaced receiver owners, timed cleanup, and redraw retirement wakes. Native window interaction and foreign receiver autoscroll remain outside this evidence.

C/PASS and A/PASS cover completed local static/core assertions only. H/NOT_RUN and M/NOT_RUN: no native or manual acceptance was attempted. This candidate must merge before shipped claims; no three-platform acceptance is inferred.

Counts: {"tests": {"passed": 5632, "ignored": 8}, "examples": {"passed": 296, "ignored": 0}, "doctests": {"passed": 20, "ignored": 1}}. Strict Clippy, warning-denied docs, host no-default-features, formatting, and diff checks passed. Complete command outcomes are in validation.json. Any compiler/linker warnings remain visible in the logs. Remaining ticket scope is tracked in Linear OPT-1363.
