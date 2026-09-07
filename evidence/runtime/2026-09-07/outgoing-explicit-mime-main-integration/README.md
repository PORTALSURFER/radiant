# outgoing-explicit-mime-main-integration — OPT-1363 partial delivery

Source: `47055ae29935fc657fa52b95d3491d1d6ce38d94`.

Combined main plus explicit MIME export after URL, cross-window, foreign autoscroll and probe prerequisites merged. MIME production/test paths preserve the previously reviewed implementation; all local checks and Windows cross-compilation reran on this exact combined source. Earlier outgoing-explicit-mime-drag evidence remains historical. Native and manual receiver acceptance remain NOT_RUN.

C/PASS and A/PASS cover completed local static/core assertions only. H/NOT_RUN and M/NOT_RUN: no native or manual acceptance was attempted. This candidate must merge before shipped claims; no three-platform acceptance is inferred.

Counts: {"tests": {"passed": 5663, "ignored": 8}, "examples": {"passed": 296, "ignored": 0}, "doctests": {"passed": 20, "ignored": 1}}. Strict Clippy, warning-denied docs, host no-default-features, formatting, and diff checks passed. Complete command outcomes are in validation.json. Any compiler/linker warnings remain visible in the logs. Remaining ticket scope is tracked in Linear OPT-1363.
