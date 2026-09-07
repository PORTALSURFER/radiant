# single-touch-drag — OPT-1363 partial delivery

Source: `f0ebc3ae07f7f9cb5c15451161f444aef97c648c`.

Single-contact typed drag uses the existing capture, preserves nonparticipant touch admission, and safely arbitrates a second contact. Commands ran at f0ebc3ae; the later merge only adds the previously reviewed modal evidence and has identical source, tests, examples, docs and Cargo inputs.

C/PASS and A/PASS cover completed local static/core assertions only. H/NOT_RUN and M/NOT_RUN: no native or manual acceptance was attempted. This candidate must merge before shipped claims; no three-platform acceptance is inferred.

Counts: {"tests": {"passed": 5615, "ignored": 8}, "examples": {"passed": 296, "ignored": 0}, "doctests": {"passed": 20, "ignored": 1}}. Strict Clippy, warning-denied docs, host no-default-features, formatting, and diff checks passed. Complete command outcomes are in validation.json. Any compiler/linker warnings remain visible in the logs. Remaining ticket scope is tracked in Linear OPT-1363.
