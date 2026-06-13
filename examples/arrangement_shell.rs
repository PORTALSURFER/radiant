//! Advanced synthetic multi-pane workspace simulation.
//!
//! This example validates Radiant workspace composition, timeline-like paint,
//! retained hover overlays, frame messages, and dense panel layout. Arrangement,
//! track, transport, mixer, audio, DSP, and plugin behavior remain host-owned
//! and are not Radiant API guidance.

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
