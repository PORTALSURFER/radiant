//! Piano-roll editor sandbox for DAW-style GUI interaction.

#[path = "piano_roll/mod.rs"]
mod piano_roll;

use piano_roll::{AppMessage, PianoRollState, project_surface, update};

fn main() -> radiant::Result {
    radiant::app(PianoRollState::default())
        .title("Radiant Piano Roll")
        .size(1040, 620)
        .min_size(820, 500)
        .view(project_surface)
        .animation(|state| state.running)
        .on_frame(|| AppMessage::Frame)
        .update(update)
        .run()
}
