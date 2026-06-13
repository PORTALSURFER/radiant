//! Advanced synthetic retained-editor simulation.
//!
//! This example validates Radiant retained canvas editing, gesture previews,
//! selection overlays, frame overlays, and dense custom-widget painting under a
//! piano-roll-shaped stress case. MIDI note editing, quantization, piano-key
//! semantics, velocity editing, and DAW workflow policy are intentionally
//! non-authoritative host-domain behavior.

#[path = "piano_roll/mod.rs"]
mod piano_roll;

use piano_roll::{AppMessage, PianoRollState, project_surface, update};
use radiant::prelude::*;

fn main() -> radiant::Result {
    radiant::app(PianoRollState::default())
        .title("Radiant Piano Roll")
        .size(1040, 620)
        .min_size(820, 500)
        .view(project_surface)
        .shortcuts(
            |_, _, press, _| match UndoRedoIntent::from_key_press(press) {
                Some(UndoRedoIntent::Undo) => ShortcutResolution::action(AppMessage::Undo),
                Some(UndoRedoIntent::Redo) => ShortcutResolution::action(AppMessage::Redo),
                None => ShortcutResolution::unhandled(),
            },
        )
        .animation(|state| state.running)
        .on_frame(|| AppMessage::Frame)
        .update(update)
        .run()
}
