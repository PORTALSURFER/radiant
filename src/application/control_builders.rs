//! Stateful control builders for the application facade.

use super::{
    MappedWidget, StateAction, ViewNode, danger_style, default_badge_sizing, default_button_sizing,
    default_drag_handle_sizing, default_selectable_sizing, default_text_input_sizing,
    default_toggle_sizing, primary_style, view_node_from_widget,
};
use crate::{
    runtime::WidgetMessageMapper,
    widgets::{
        BadgeWidget, ButtonWidget, DragHandleWidget, SelectableWidget, TextInputWidget,
        ToggleWidget, WidgetProminence, WidgetStyle,
    },
};
use std::sync::Arc;

mod badge;
mod button;
mod drag_handle;
mod selectable;
mod text_input;
mod toggle;

pub use badge::{BadgeBuilder, badge, badge_mapped, badge_message};
pub use button::{ButtonBuilder, button, button_mapped, button_message};
pub use drag_handle::{DragHandleBuilder, drag_handle, drag_handle_mapped};
pub use selectable::{SelectableBuilder, selectable, selectable_mapped};
pub use text_input::{TextInputBuilder, text_input, text_input_mapped};
pub use toggle::{ToggleBuilder, checkbox, toggle, toggle_mapped};
