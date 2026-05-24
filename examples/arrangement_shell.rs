//! Arrangement shell sandbox for DAW-style multi-pane GUI composition.

#[path = "arrangement_shell/mod.rs"]
mod arrangement_shell;

use arrangement_shell::{AppMessage, ArrangementShellState, project_surface, update};

fn main() -> radiant::Result {
    radiant::app(ArrangementShellState::default())
        .title("Radiant Arrangement Shell")
        .size(1180, 700)
        .min_size(900, 560)
        .view(project_surface)
        .animation(|state| state.running)
        .on_frame(|| AppMessage::Frame)
        .update(update)
        .run()
}
