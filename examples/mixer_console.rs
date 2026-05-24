//! Eight-channel mixer console sandbox for DAW-style GUI interaction.

#[path = "mixer_console/mod.rs"]
mod mixer_console;

use mixer_console::{MixerMessage, MixerState, project_surface, update};

fn main() -> radiant::Result {
    radiant::app(MixerState::default())
        .title("Radiant Mixer Console")
        .size(1040, 620)
        .min_size(820, 500)
        .view(project_surface)
        .animation(|state| state.running)
        .on_frame(|| MixerMessage::Frame)
        .update(update)
        .run()
}
