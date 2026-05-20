//! Horizontal SVG-icon toolbar with state-driven toggle buttons.

use radiant::prelude::*;
use std::sync::Arc;

#[path = "toolbar_icons/icon_button.rs"]
mod icon_button;
#[path = "toolbar_icons/icons.rs"]
mod icons;
#[path = "toolbar_icons/model.rs"]
mod model;
#[path = "toolbar_icons/view.rs"]
mod view;

#[cfg(test)]
#[path = "toolbar_icons/tests.rs"]
mod tests;

use model::{ToolMessage, ToolbarState};
use view::project_surface;

fn main() -> radiant::Result {
    radiant::app(ToolbarState::default())
        .title("Radiant Toolbar Icons")
        .size(360, 150)
        .min_size(300, 120)
        .view(project_surface)
        .update(|state, message| match message {
            ToolMessage::Toggle(tool) => state.toggle(tool),
        })
        .run()
}
