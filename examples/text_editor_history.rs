//! Application-owned undo/redo for a controlled [`TextEditorDocument`].
//!
//! Radiant attaches transient grouping metadata to `TextEditorEdit`; it never
//! retains values or durable history. This headless fixture keeps snapshots in
//! the application and deliberately resets the document through `set_text` for
//! undo and redo.

use radiant::{
    application::{IntoView, TextEditorDocument, TextEditorEdit, text_editor},
    runtime::{Event, RuntimeBridge, SurfaceRuntime, UiSurface},
    widgets::interaction::{EditPhase, TextEditGroupEvent, TextEditKind},
    widgets::{CompositionRange, CompositionSample, TextEditCommand, WidgetInput},
};
use std::sync::Arc;

const EDITOR_ID: u64 = 77;

enum Message {
    Edit(TextEditorEdit),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HistoryEntry {
    before: String,
    after: String,
    kind: TextEditKind,
}

/// This is application state. It is intentionally not a Radiant widget model.
struct HistoryApp {
    document: TextEditorDocument,
    undo: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
    active: Option<(radiant::widgets::EditTransaction, String, TextEditKind)>,
}

impl HistoryApp {
    fn new() -> Self {
        Self {
            document: TextEditorDocument::new("").expect("bounded fixture document"),
            undo: Vec::new(),
            redo: Vec::new(),
            active: None,
        }
    }

    fn finish(&mut self, event: TextEditGroupEvent, fallback_before: String, after: String) {
        let (transaction, before, kind) =
            self.active
                .take()
                .unwrap_or((event.transaction, fallback_before, event.kind));
        if transaction == event.transaction && before != after && event.phase != EditPhase::Cancel {
            self.undo.push(HistoryEntry {
                before,
                after,
                kind,
            });
            self.redo.clear();
        }
    }

    fn reduce_edit(&mut self, edit: TextEditorEdit) {
        let grouping = edit.grouping();
        let before = self.document.text().to_owned();

        // An `ended` event belongs to the text before the incoming edit. This
        // keeps typing and a following atomic paste as distinct history entries.
        if let Some(ended) = grouping.ended {
            self.finish(ended, before.clone(), before.clone());
        }
        self.document.apply(&edit).expect("current controlled edit");
        if let Some(current) = grouping.current {
            match current.phase {
                EditPhase::Begin => {
                    self.active = Some((current.transaction, before, current.kind));
                }
                EditPhase::Commit => self.finish(current, before, self.document.text().to_owned()),
                EditPhase::Cancel => self.active = None,
                EditPhase::Update => {}
            }
        }
    }

    fn undo(&mut self) {
        if let Some((_, before, kind)) = self.active.take()
            && before != self.document.text()
        {
            self.undo.push(HistoryEntry {
                before,
                after: self.document.text().to_owned(),
                kind,
            });
        }
        if let Some(entry) = self.undo.pop() {
            self.document
                .set_text(entry.before.clone())
                .expect("bounded undo");
            self.redo.push(entry);
        }
    }

    fn redo(&mut self) {
        if let Some(entry) = self.redo.pop() {
            self.document
                .set_text(entry.after.clone())
                .expect("bounded redo");
            self.undo.push(entry);
        }
    }
}

impl RuntimeBridge<Message> for HistoryApp {
    fn project_surface(&mut self) -> Arc<UiSurface<Message>> {
        text_editor(self.document.snapshot())
            .id(EDITOR_ID)
            .message(Message::Edit)
            .into_surface()
            .into()
    }

    fn reduce_message(&mut self, message: Message) {
        let Message::Edit(edit) = message;
        self.reduce_edit(edit);
    }
}

fn input(runtime: &mut SurfaceRuntime<HistoryApp, Message>, command: TextEditCommand) {
    assert!(
        runtime
            .dispatch_focused_input(WidgetInput::text_edit(command))
            .is_some()
    );
}

fn run_fixture() -> (String, usize, usize) {
    let mut runtime = SurfaceRuntime::new(HistoryApp::new(), Default::default());
    assert!(runtime.focus_widget(EDITOR_ID));
    input(&mut runtime, TextEditCommand::InsertText("a".into()));
    input(&mut runtime, TextEditCommand::InsertText("b".into()));
    input(&mut runtime, TextEditCommand::InsertText("c".into()));
    // Clipboard edits end typing and commit atomically.
    input(&mut runtime, TextEditCommand::PasteText("!".into()));
    assert_eq!(runtime.bridge().undo.len(), 2);
    assert_eq!(runtime.bridge().document.text(), "abc!");

    // A composition session is one committed history entry; canceled preedit is
    // transient and never becomes a durable snapshot.
    let at_end = CompositionRange::new(4, 4, 4).expect("collapsed composition range");
    assert_eq!(
        runtime.dispatch_composition_sample(
            CompositionSample::start(at_end, at_end).expect("composition start"),
        ),
        Some(EDITOR_ID)
    );
    let preedit = CompositionRange::new(4, 5, 5).expect("composition selection");
    assert_eq!(
        runtime.dispatch_composition_sample(
            CompositionSample::update("x", preedit).expect("composition update"),
        ),
        Some(EDITOR_ID)
    );
    assert_eq!(
        runtime.dispatch_composition_sample(CompositionSample::commit("x")),
        Some(EDITOR_ID)
    );
    assert_eq!(runtime.bridge().undo.len(), 3);
    let at_end = CompositionRange::new(5, 5, 5).expect("collapsed composition range");
    assert!(
        runtime
            .dispatch_composition_sample(
                CompositionSample::start(at_end, at_end).expect("cancel start"),
            )
            .is_some()
    );
    assert!(
        runtime
            .dispatch_composition_sample(
                CompositionSample::update(
                    "ignored",
                    CompositionRange::new(5, 12, 12).expect("cancel update")
                )
                .expect("cancel preedit"),
            )
            .is_some()
    );
    assert!(
        runtime
            .dispatch_composition_sample(CompositionSample::cancel())
            .is_some()
    );
    assert_eq!(runtime.bridge().undo.len(), 3);

    runtime.bridge_mut().undo();
    assert_eq!(runtime.bridge().document.text(), "abc!");
    runtime.bridge_mut().undo();
    assert_eq!(runtime.bridge().document.text(), "abc");
    runtime.bridge_mut().undo();
    assert_eq!(runtime.bridge().document.text(), "");
    runtime.bridge_mut().redo();
    runtime.bridge_mut().redo();
    runtime.bridge_mut().redo();
    (
        runtime.bridge().document.text().to_owned(),
        runtime.bridge().undo.len(),
        runtime.bridge().redo.len(),
    )
}

fn main() {
    let (text, undo, redo) = run_fixture();
    println!("{{\"text\":\"{text}\",\"undo\":{undo},\"redo\":{redo}}}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_groups_and_clipboard_is_atomic_with_application_owned_redo() {
        assert_eq!(run_fixture(), ("abc!".into(), 2, 0));
    }
}
