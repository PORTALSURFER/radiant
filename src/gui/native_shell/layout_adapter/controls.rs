//! Slotized helpers for native-shell action-button rows and toolbar partitions.

#[path = "controls/browser_toolbar.rs"]
mod browser_toolbar;
#[path = "controls/shared.rs"]
mod shared;
#[path = "controls/update_buttons.rs"]
mod update_buttons;

pub(crate) use browser_toolbar::compute_browser_toolbar_sections;
pub(crate) use update_buttons::compute_update_action_button_rects;
