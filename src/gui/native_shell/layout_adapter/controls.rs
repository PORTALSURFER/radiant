//! Slotized helpers for native-shell action-button rows and toolbar partitions.

#[path = "controls/content_toolbar.rs"]
mod content_toolbar;
#[path = "controls/shared.rs"]
mod shared;
#[path = "controls/update_buttons.rs"]
mod update_buttons;

pub(crate) use content_toolbar::compute_content_toolbar_sections;
pub(crate) use update_buttons::compute_update_action_button_rects;
