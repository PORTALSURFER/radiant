//! Runtime builder helpers for selectable primitives.

use crate::runtime::{PaintText, SurfaceNode, WidgetMessageMapper};
use crate::widgets::contract::{WidgetId, WidgetSizing};
use crate::widgets::interaction::SelectableMessage;

use super::SelectableWidget;

impl<Message> WidgetMessageMapper<Message> {
    /// Build a selectable-message mapper.
    pub fn selectable(map: impl Fn(SelectableMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a selectable leaf that maps selection changes by selected state.
    pub fn selectable(
        id: WidgetId,
        label: impl Into<String>,
        selected: bool,
        sizing: WidgetSizing,
        map: impl Fn(bool) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::selectable_mapped(id, label, selected, sizing, move |message| match message {
            SelectableMessage::SelectionChanged { selected } => map(selected),
        })
    }

    /// Build a selectable leaf with a custom widget-to-host message mapper.
    pub fn selectable_mapped(
        id: WidgetId,
        label: impl Into<String>,
        selected: bool,
        sizing: WidgetSizing,
        map: impl Fn(SelectableMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            SelectableWidget::new(id, PaintText::from(label.into()), selected, sizing),
            WidgetMessageMapper::selectable(map),
        )
    }
}
