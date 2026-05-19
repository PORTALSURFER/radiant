//! Runtime builder helpers for toggle primitives.

use crate::runtime::{PaintText, SurfaceNode, WidgetMessageMapper};
use crate::widgets::contract::{WidgetId, WidgetSizing};
use crate::widgets::interaction::ToggleMessage;

use super::ToggleWidget;

impl<Message> WidgetMessageMapper<Message> {
    /// Build a toggle-message mapper.
    pub fn toggle(map: impl Fn(ToggleMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a toggle leaf that maps value changes by checked state.
    pub fn toggle(
        id: WidgetId,
        label: impl Into<String>,
        sizing: WidgetSizing,
        map: impl Fn(bool) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::toggle_with_checked(id, label, false, sizing, map)
    }

    /// Build a toggle leaf with an explicit checked state.
    pub fn toggle_with_checked(
        id: WidgetId,
        label: impl Into<String>,
        checked: bool,
        sizing: WidgetSizing,
        map: impl Fn(bool) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::toggle_mapped_with_checked(id, label, checked, sizing, move |message| match message {
            ToggleMessage::ValueChanged { checked } => map(checked),
        })
    }

    /// Build a toggle leaf with a custom widget-to-host message mapper.
    pub fn toggle_mapped(
        id: WidgetId,
        label: impl Into<String>,
        sizing: WidgetSizing,
        map: impl Fn(ToggleMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::toggle_mapped_with_checked(id, label, false, sizing, map)
    }

    /// Build a toggle leaf with explicit checked state and a custom mapper.
    pub fn toggle_mapped_with_checked(
        id: WidgetId,
        label: impl Into<String>,
        checked: bool,
        sizing: WidgetSizing,
        map: impl Fn(ToggleMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            ToggleWidget::new(id, PaintText::from(label.into()), sizing).with_checked(checked),
            WidgetMessageMapper::toggle(map),
        )
    }
}
