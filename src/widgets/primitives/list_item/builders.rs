//! Runtime builder helpers for list-item primitives.

use crate::runtime::{PaintText, SurfaceNode, WidgetMessageMapper};
use crate::widgets::contract::{WidgetId, WidgetSizing};
use crate::widgets::interaction::ListItemMessage;

use super::ListItemWidget;

impl<Message> WidgetMessageMapper<Message> {
    /// Build a list-item-message mapper.
    pub fn list_item(map: impl Fn(ListItemMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a non-emitting list item leaf node.
    pub fn list_item(id: WidgetId, label: impl Into<String>, sizing: WidgetSizing) -> Self {
        Self::static_widget(ListItemWidget::new(
            id,
            PaintText::from(label.into()),
            sizing,
        ))
    }

    /// Build an invoking list item leaf node that emits one cloned host message.
    pub fn list_item_action(
        id: WidgetId,
        label: impl Into<String>,
        sizing: WidgetSizing,
        message: Message,
    ) -> Self
    where
        Message: Clone + Send + Sync + 'static,
    {
        Self::list_item_mapped(id, label, sizing, move |_| message.clone())
    }

    /// Build an invoking list item leaf node with a custom widget-to-host message mapper.
    pub fn list_item_mapped(
        id: WidgetId,
        label: impl Into<String>,
        sizing: WidgetSizing,
        map: impl Fn(ListItemMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            ListItemWidget::new(id, PaintText::from(label.into()), sizing),
            WidgetMessageMapper::list_item(map),
        )
    }
}
