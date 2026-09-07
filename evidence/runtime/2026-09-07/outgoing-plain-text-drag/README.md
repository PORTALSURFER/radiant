# outgoing-plain-text-drag — OPT-1363 partial delivery

Source: `0c2476c82d84e00a4f5c4512084054fdb5d23dcd`.

Outgoing plain-text requests validate a 1 MiB UTF-8 bound and reject embedded NUL before native launch. macOS uses an owned pasteboard writer and Windows uses CF_UNICODETEXT with bounded CRLF-normalized UTF-16 allocation. Existing source identity and one-shot completion behavior are preserved. Portable encoding, invalid native launch, and controller lifecycle tests passed; no actual OS drag or clipboard interaction was attempted.

C/PASS and A/PASS cover completed local static/core assertions only. H/NOT_RUN and M/NOT_RUN: no native or manual acceptance was attempted. This candidate must merge before shipped claims; no three-platform acceptance is inferred.

Counts: {"tests": {"passed": 5622, "ignored": 8}, "examples": {"passed": 296, "ignored": 0}, "doctests": {"passed": 20, "ignored": 1}}. Strict Clippy, warning-denied docs, host no-default-features, formatting, and diff checks passed. Complete command outcomes are in validation.json. Any compiler/linker warnings remain visible in the logs. Remaining ticket scope is tracked in Linear OPT-1363.
