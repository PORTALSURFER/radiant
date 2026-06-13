//! Advanced synthetic matrix simulation.
//!
//! This example validates Radiant dense matrix interaction, value editing,
//! synthetic activity markers, and paint-only hover overlays. Synth modulation
//! routing semantics are intentionally non-authoritative host-domain behavior.

#[path = "modulation_matrix/mod.rs"]
mod modulation_matrix;

use modulation_matrix::{AppMessage, ModulationMatrixState, project_surface, update};

fn main() -> radiant::Result {
    radiant::app(ModulationMatrixState::default())
        .title("Radiant Modulation Matrix")
        .size(1040, 620)
        .min_size(820, 500)
        .view(project_surface)
        .animation(|state| state.running)
        .on_frame(|| AppMessage::Frame)
        .update(update)
        .run()
}
