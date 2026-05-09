//! Shared support for primitive widget implementations.

mod common;
mod input;
mod paint;

pub use common::WidgetCommon;
pub(super) use input::{activate_on_keyboard, clamp_fraction};
pub(super) use paint::{push_button_chrome, push_checkbox_chrome, push_control_chrome};
