//! Advanced synthetic dense-panel simulation.
//!
//! This example validates Radiant dense custom-widget paint, fader-like drags,
//! grouped previews, row reordering, and paint-only hover overlays. Mixer,
//! channel, send, solo, mute, and DSP semantics are intentionally
//! non-authoritative host-domain behavior.

#[path = "mixer_console/mod.rs"]
mod mixer_console;

use mixer_console::{MixerMessage, MixerState, project_surface, update};

fn main() -> radiant::Result {
    radiant::app(MixerState::default())
        .title("Radiant Mixer Console")
        .size(1440, 760)
        .min_size(1180, 620)
        .view(project_surface)
        .animation(|state| state.running)
        .on_frame(|| MixerMessage::Frame)
        .update(update)
        .run()
}
