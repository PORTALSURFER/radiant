# Editor clipboard, privacy, and transient history evidence

Final source: `a821309544399c0bcfed1ab7ee58bcacaa29ed89`, based on multiline-editor merge `799c5816aa459a6cf47d5c59b2c3a802ac295dc1`.

The complete suite at `399eec7e` passed 4,409 library tests (8 ignored), 1,092 integration tests, and 294 example tests. The final source changes only box rejected clipboard jobs and mapped completion payloads to satisfy strict Clippy. Its 16 focused clipboard regressions, all-target/all-feature Clippy, documentation, doctests, no-default-features library build, and controlled history fixture pass. Formatting and diff whitespace checks pass at final source.

The history fixture reports `{"text":"abc!x","undo":3,"redo":0}` after three application-owned undo and redo operations; every restoration refreshes and checks the actual projected editor. Radiant retains only transient group identity and editor state, not a durable history stack.

Independent reviews covered retained authority and privacy, Unicode display/source mapping, transient grouping, and the serialized clipboard worker lifecycle. Findings in the history expectation/preedit ranges, secret mapping allocation, and debug redaction were corrected. Full verification also caught and corrected the native Enter/Tab compatibility regression and the stale documentation guardrail requiring widget-owned history.

The platform lane fixtures use fake clipboard backends, and native keyboard tests do not read or overwrite the OS clipboard. Headless/offscreen tests do not claim foreground clipboard or Japanese/Chinese IME acceptance. Native platform acceptance remains explicitly separate from these deterministic lifecycle checks. Existing ignored tests and the toolchain compact-unwind linker warning are not presented as passing acceptance.
