//! Runtime builder helpers for badge primitives.

use crate::runtime::{PaintText, SurfaceNode, WidgetMessageMapper};
use crate::widgets::contract::{WidgetId, WidgetSizing};
use crate::widgets::interaction::BadgeMessage;

use super::BadgeWidget;

impl<Message> WidgetMessageMapper<Message> {
    /// Build a badge-message mapper.
    pub fn badge(map: impl Fn(BadgeMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }

    /// Build an allocation-free badge activation binding.
    pub(crate) fn badge_message(message: Message) -> Self
    where
        Message: Clone + Send + Sync + 'static,
    {
        Self::constant(message, |output| {
            matches!(
                output.typed_ref::<BadgeMessage>(),
                Some(BadgeMessage::Activate)
            )
        })
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a badge or pill leaf node that emits one cloned host message when activated.
    pub fn badge(
        id: WidgetId,
        label: impl Into<String>,
        sizing: WidgetSizing,
        message: Message,
    ) -> Self
    where
        Message: Clone + Send + Sync + 'static,
    {
        Self::widget(
            BadgeWidget::new(id, PaintText::from(label.into()), sizing),
            WidgetMessageMapper::badge_message(message),
        )
    }

    /// Build a badge or pill leaf node with a custom widget-to-host message mapper.
    pub fn badge_mapped(
        id: WidgetId,
        label: impl Into<String>,
        sizing: WidgetSizing,
        map: impl Fn(BadgeMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            BadgeWidget::new(id, PaintText::from(label.into()), sizing),
            WidgetMessageMapper::badge(map),
        )
    }
}
