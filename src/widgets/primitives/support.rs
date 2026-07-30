//! Shared support for primitive widget implementations.

mod common;
mod input;
mod paint;
pub(super) mod revision;

pub use common::WidgetCommon;
pub(super) use input::{activate_on_keyboard, clamp_fraction};
pub(super) use paint::{
    push_automation_active_marker, push_button_chrome, push_button_focus_ring,
    push_checkbox_chrome, push_control_chrome, push_selected_active_marker,
};
