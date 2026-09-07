# Text clipboard, privacy, and transient grouping

Text values and durable history belong to the application. Radiant keeps transient caret, selection, composition, geometry, and text-free group identity. It does not keep an undo stack.

## Deferred clipboard admission

`SurfaceRuntime::begin_focused_text_clipboard` admits a typed `TextClipboardOperation::{Copy, Cut, Paste}` for the authoritative focused text control. Native C/X/V shortcuts use the same path. A focused builtin text control consumes these shortcuts even when a particular operation is denied or the platform service is unavailable.

Admission captures exact widget/actor authority, privacy policy, and text/selection state. Multiline receipts also carry weak document authority and revision; application `set_text`, an accepted document edit, or document destruction invalidates pending work even before reprojection. Clipboard receipts never keep a document owner alive.

Requests use the existing qualified platform result validation and bounded deferred ingress. A successful copy has no text mutation. Cut deletes only after a successful clipboard write and a second exact-state check. Paste inserts only a valid text result into the still-current selection. Focus loss, replacement/removal, composition changes, policy changes, revision changes, cancellation, and runtime shutdown make late results inert. A synchronous custom host completion is still delivered on a later drain turn.

External text payloads are limited to 16 KiB. A single-line receipt captures at most 1 MiB of source state for exact comparison. Existing controller and application completion reservations are bounded at 64. The application adapter uses a lazy serial clipboard worker with a 64-slot queue, retaining its native clipboard backend until shutdown. Clipboard requests do not depend on business-worker availability. Creation, access, and destruction of the native clipboard happen on that worker; UI shutdown does not join it. Cancellation is checked immediately before native work, but cannot undo an OS call that has already begun.

Clipboard request/response/value Debug output reports structure and sizes, not text or copied paths. The adapter emits fixed failure descriptions rather than payload-bearing platform error strings.

## Secret presentation

Use `TextInputBuilder::privacy`, `TextEditorBuilder::privacy`, or the corresponding widget `with_privacy` method with `TextPrivacy::Secret(TextSecretPolicy::new())`. Privacy types are available from `radiant::widgets`.

Secret mode denies copy/cut and automation exposure by default. `allow_copy()` and `allow_automation()` are independent explicit opt-ins. Paste remains available for an editable focused field. Secret semantics include deterministic `text.privacy`, `text.copy_allowed`, and `text.automation_allowed` metadata. They omit the value by default; automation writes/actions are denied unless allowed.

Masking uses one bullet per extended grapheme cluster and normalizes hard separators to line breaks. The mapping is bounded to 1 MiB and 65,536 graphemes and retains masked text plus byte-boundary offsets, not a second source-text value. If a secret presentation cannot be represented within the bound, it fails closed with empty display content. Single-line completion suffixes are omitted in secret mode.

Masking happens before paint plans, native shaping caches, and semantic snapshots. Selection and caret geometry use display offsets; accepted hits map back to exact source boundaries. Combining sequences and ZWJ emoji therefore do not gain interior cursor stops merely because they are masked. IME preedit uses the same privacy mapping. Application-facing values and typed edits still carry the real text so the application can own its data.

## Application-owned history

`TextEditorEdit::grouping()` returns `TextEditGrouping`. Consecutive compatible typing/deletion edits share an opaque transaction. Navigation, focus loss, a change of edit kind, and clipboard operations break continuity. Composition begins, updates, commits, or cancels one group. A paste/cut is an atomic Commit event. A grouping record can close the previous group and begin the next on the same typed edit.

These identities are process-local and must not be persisted. Metadata contains no text, durable history, clock heuristic, or undo policy. Direct application-created document edits have empty grouping metadata. External application resets establish a new authority; the application closes or replaces its own active history group as part of that reset.

Single-line applications can opt into grouped events with `TextInputBuilder::edit_message`; existing `message` and `message_event` callbacks retain their legacy behavior. The [application-owned history example](../examples/text_editor_history.rs) consumes typed edits and grouping events, owns before/after values, and implements undo/redo by publishing fresh document authority with `set_text`.

## Validation scope

Deterministic widget and controller tests cover privacy snapshots, Unicode boundary mapping, composition, copy policy, failed cut, deferred paste, replacement/revision/focus/close races, and bounded worker behavior. The clipboard lane tests use a fake backend and do not read or overwrite the user's system clipboard. Native IME acceptance is tracked separately from these model and adapter lifecycle checks.
