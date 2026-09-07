# OPT-1392 feedback and notifications

Source: `39276245f1c67544dba5f8919117fb9bfec7612f`, based on main `d6bec66a6d466cb9797b6297a246c086edf1a250`.

The application owns a bounded queue and handles exact-owner action/dismissal requests. Accepted projections use one runtime deadline; hover, focus, modal, visibility, and recovery pause remaining time. Passive feedback uses the shared animation runtime.

Validation was serialized with the shared Cargo target and incremental compilation disabled. Full library tests passed at `352cc8eb` (4331 passed, 8 ignored). Subsequent changes only gate obsolete constructors to tests, make invalid feedback declarations fall back to static paint, and fix strict lint checks. Integration tests passed (1092), example tests passed (291 including 13 feedback lifecycle scenarios), and doctests passed (20, 1 ignored) before the final lint-only change. Documentation, strict all-target/all-feature Clippy, no-default library check, formatting, diff checks, and the deterministic example passed with the final source. The final focused feedback test is recorded separately.

Independent review found captured-pointer hover accounting, corrected before validated routing. A full library run exposed recursive surface stack growth from additional metadata handles; resource and notice demands now share one optional metadata handle, preserving the original footprint. The unchanged deep-boundary regression and full library suite pass. Both fixes received independent review. A final primary review covered the small fixture, static fallback, and lint follow-ups.

The headless example reports one dismissal after expiry. Tests cover ignored expiry, critical persistence, replacement/removal, queued admission, hover/focus/modal/hidden pauses, semantic labels, reduced motion, action dispatch, and incoming-notice focus preservation. No foreground window or native performance measurement was used or claimed.
