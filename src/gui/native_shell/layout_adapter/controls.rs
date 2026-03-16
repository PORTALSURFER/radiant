//! Slotized helpers for native-shell action-button rows and toolbar partitions.

mod browser_toolbar;
mod shared;
mod sidebar_buttons;
mod update_buttons;

pub(crate) use browser_toolbar::compute_browser_toolbar_sections;
pub(crate) use sidebar_buttons::compute_sidebar_action_button_rects;
pub(crate) use update_buttons::compute_update_action_button_rects;

#[cfg(test)]
#[path = "controls_tests.rs"]
mod controls_tests;
