//! Normal-window launcher for a real floating popup Radiant surface.
//!
//! Run `cargo run --example popup_window`, then use the main window controls to
//! reveal a second borderless popup window. Drag the popup from its title area
//! to reposition it, or hide it from the popup itself. The popup is the same
//! example binary relaunched with `--popup`. After the launcher window starts,
//! it prepares one offscreen popup host for every mode, so the user-facing open
//! path only moves and focuses an already prepared native surface.

use radiant::prelude::*;
use std::time::Duration;

#[path = "popup_window/host.rs"]
mod host;
#[path = "popup_window/launcher.rs"]
mod launcher;
#[path = "popup_window/model.rs"]
mod model;
#[path = "popup_window/policy.rs"]
mod policy;
#[path = "popup_window/popup.rs"]
mod popup;

#[cfg(test)]
#[path = "popup_window/tests.rs"]
mod tests;

use launcher::run_launcher_window;
use model::popup_launch_from_args;
use popup::run_popup_window;

fn main() -> radiant::Result {
    if popup_launch_from_args().is_some() {
        run_popup_window()
    } else {
        run_launcher_window()
    }
}
