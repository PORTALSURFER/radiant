//! Paint projection helpers for primitive widget implementations.

mod chrome;

pub(in crate::widgets::primitives) use chrome::{
    push_automation_active_marker, push_button_chrome, push_button_focus_ring,
    push_checkbox_chrome, push_control_chrome, push_selected_active_marker,
};
