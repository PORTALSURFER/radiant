# Modal gesture retirement — OPT-1394 follow-up

Source: `97440e4daa7c25ae880895aa0ba5ad93dc1d0a32`.

Modal publication retires background gesture capture and pending touch contacts through the existing input lifecycle.

C/PASS and A/PASS cover completed local static/core assertions only. H/NOT_RUN and M/NOT_RUN: no native or manual acceptance was attempted. This candidate must merge before shipped claims; no three-platform acceptance is inferred.

Counts: {"tests": {"passed": 5575, "ignored": 8}, "examples": {"passed": 295, "ignored": 0}, "doctests": {"passed": 20, "ignored": 1}}. Strict Clippy, warning-denied docs, host no-default-features, formatting, and diff checks passed. Complete command outcomes are in validation.json. Any compiler/linker warnings remain visible in the logs. Native overlay acceptance remains tracked in Linear OPT-1394; broader drag-and-drop scope remains in OPT-1363.
