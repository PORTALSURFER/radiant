# outgoing-explicit-mime-drag — OPT-1363 partial delivery

Source: `3dbe1004821c4bba3fad7443fdf776c9372f6ad7`.

Explicit bounded MIME export preserves caller bytes. macOS uses MIME-derived UTI and NSData; Windows advertises exact-length IStream and preserves existing HGLOBAL file/text/URL transports. Windows all-target/all-feature cross-compilation passed on macOS; this is not native Windows execution. Native and manual drop acceptance remain NOT_RUN.

C/PASS and A/PASS cover completed local static/core assertions only. H/NOT_RUN and M/NOT_RUN: no native or manual acceptance was attempted. This candidate must merge before shipped claims; no three-platform acceptance is inferred.

Counts: {"tests": {"passed": 5632, "ignored": 8}, "examples": {"passed": 296, "ignored": 0}, "doctests": {"passed": 20, "ignored": 1}}. Strict Clippy, warning-denied docs, host no-default-features, formatting, and diff checks passed. Complete command outcomes are in validation.json. Any compiler/linker warnings remain visible in the logs. Remaining ticket scope is tracked in Linear OPT-1363.
