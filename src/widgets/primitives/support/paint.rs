//! Paint projection helpers for primitive widget implementations.

mod chrome;

pub(in crate::widgets::primitives) use chrome::{
    push_automation_active_marker, push_button_chrome, push_checkbox_chrome, push_control_chrome,
};
