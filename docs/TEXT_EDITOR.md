# Multiline Text Editor

`text_editor(snapshot)` is a controlled multiline editor. The application owns
its `TextEditorDocument`, projects `document.snapshot()`, maps widget output to
`TextEditorEdit`, applies an accepted edit to that same document, and then
reprojects. The headless [`text_editor` example](../examples/text_editor.rs)
demonstrates the complete route without opening a native window.

```rust
enum Message {
    Edit(radiant::application::TextEditorEdit),
}

fn update(state: &mut State, message: Message) {
    let Message::Edit(edit) = message;
    state.document.apply(&edit).expect("current document edit");
}
```

## Document authority

Documents accept at most 1 MiB of UTF-8 text and 65,536 graphemes. Every
document receives a globally monotonic revision, and every `TextEditorEdit`
carries the exact document owner plus its expected and resulting revisions.
`apply` rejects an edit for another owner or a stale revision. Treat edits as
single-use deltas: do not reconstruct them and do not apply them to a copied or
replacement document.

`set_text` publishes a newer external document authority, including an explicit
same-value reset. It clears selection and composition. Application state owns
undo/redo history, persistence, collaboration policy, and any decision to
replace text; Radiant does not retain that product history.

## Selection and composition

Selection is part of every revision and uses grapheme boundaries. Composition
updates are also exact edits, but their preedit text is only display text:
`snapshot.text()` and `document.text()` remain committed text until a
composition commit. A cancel restores the composition's original selection.
Applications should reproject each accepted selection or composition delta so
the widget and host keep the same revision.

## Geometry receipts

Before pointer, keyboard, wheel, or IME routing, a host shapes the current
editor declaration and installs a `TextEditorGeometryReceipt` through
`SurfaceRuntime::install_text_editor_geometry`. A receipt is valid only for the
exact widget, document owner, revision, displayed text, bounds, wrapping,
font, direction, locale, and line height it declares. It is shared by paint,
hit testing, navigation, scrolling, and IME geometry.

The example's ASCII paragraph geometry is deliberately deterministic and
headless. It is not a production text provider; native hosts must supply their
own shaping result for each accepted declaration. Reflow, text-scale changes,
wrap changes, and edits require a fresh matching receipt.

Native hosts seed focused editor geometry first and admit an idle editor's exact
current declaration before pointer or wheel dispatch. The shaping cache retains
at most eight paragraphs; accepted-plan retention is capped at 64 editors and
32 MiB. Idle entries can be evicted and readmitted, so an editor beyond the
initial retention count still receives input. Painting uses the retained paragraph
when its declaration matches the accepted plan.

## Bounded reflow scenario

`cargo bench --locked --bench perf_harness text_editor_reflow_64k -- --jsonl`
measures renderer-neutral reflow at the 65,536-grapheme document limit, alternating
320- and 640-unit wrap widths and resolving the final caret. It includes geometry
input cloning and construction. It excludes native shaping, GPU encoding, and
platform IME latency; those require separate host evidence.

## Clipboard, secret mode, and history grouping

See [text editing privacy and grouping](TEXT_EDITING_PRIVACY.md) for deferred
clipboard admission, secret masking, and `TextEditorEdit::grouping()`.
The [history example](../examples/text_editor_history.rs) keeps undo/redo values
in application state and restores them through fresh document revisions.
