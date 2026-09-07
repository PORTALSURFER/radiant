# OPT-1404 multiline TextEditor

Final source: `481ac01a29e806151f9be60c39594270759daa53`. Full quality source before the equivalent range-lookup optimization: `9c8dc7c661ba5e96383929b8d47d68ce8c06450c`. Base main: `7745a1671e7f4615a1ecdc5f248c164d28a91330`.

The application owns the document and accepts typed exact-owner/revision edits. Shared bounded paragraph geometry governs wrapping, caret/navigation/selection, scroll reveal, native painting, and IME caret placement. Native plan retention admits editors beyond the initial 64 retained entries before input.

Validation used serialized Cargo work with incremental compilation disabled. Full library tests: 4371 passed, 8 ignored. Integration tests: 1092 passed. Example tests: 293 passed. Doctests: 20 passed, 1 ignored. Documentation, no-default library check, strict all-target/all-feature Clippy, formatting, and the headless application fixture passed. The final change replaces a full grapheme-set scan with the equivalent inclusive BTreeSet range; final paragraph tests (14), formatting and strict Clippy passed again.

Independent review covered native retention, RTL ligature origins, controlled reprojection, focus restoration during captured dragging, and exact geometry wheel admission. Review findings were fixed and regressions added. Review also confirmed that the final range lookup preserves inclusive caret validation.

The release benchmark alternates 320/640-unit wrapping for 65536 ASCII graphemes, including geometry input cloning, geometry construction, and final caret resolution. Twenty measured iterations averaged 10784961.100 microseconds before the range fix and 16292.750 microseconds afterward. This is a local algorithm comparison, not a native host latency claim; it excludes native font shaping, GPU encoding, foreground interaction, and actual Japanese/Chinese IME acceptance. Native IME acceptance is tracked separately.
