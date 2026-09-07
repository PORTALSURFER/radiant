# Two-contact gesture validation — OPT-1363 partial delivery

Source: `d2ab83d695a784e7c72ababc56c85aed562399db`. Candidate source was tested locally and committed unchanged apart from documentation/comments and formatting; examples and all-target lint ran after the final example update.

C/PASS: strict all-target/all-feature Clippy, documentation, doctests, host no-default-features build, formatting and diff checks.
A/PASS: 5519 library/integration tests (8 existing ignored), 295 example tests, and 12 focused public touch regressions. Native adapter fixtures are deterministic A evidence.
H/NOT_RUN and M/NOT_RUN: no real native event-loop touch or physical device acceptance was attempted. This bundle does not claim either lane or three-platform acceptance.

The slice derives pan, pinch and rotation from admitted same-device contact tokens using the existing gesture owner. Tests cover baseline rebasing, deterministic arbitration, final terminal geometry, cancellation, stale contacts, source/policy replacement, and native gesture isolation. Target styling/autoscroll, cross-window payloads and external offers remain outstanding ticket scope. Source is not shipped until canonical merge.

The library linker emitted the existing compact-unwind table-size warning; no test failed. All payload paths are relative to this folder; manifests record their SHA-256 checksums.
