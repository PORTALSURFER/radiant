//! Runtime builder helpers for button primitives.

use crate::runtime::{PaintText, SurfaceNode, WidgetMessageMapper};
use crate::widgets::contract::{WidgetId, WidgetSizing};
use crate::widgets::interaction::ButtonMessage;

use super::ButtonWidget;

impl<Message> WidgetMessageMapper<Message> {
    /// Build a button-message mapper.
    pub fn button(map: impl Fn(ButtonMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }

    /// Build an allocation-free button activation binding.
    pub(crate) fn button_message(message: Message) -> Self
    where
        Message: Clone + Send + Sync + 'static,
    {
        Self::constant(message, |output| {
            output
                .typed_copied::<ButtonMessage>()
                .is_some_and(ButtonMessage::is_activate)
        })
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a button leaf node that emits one cloned host message when activated.
    pub fn button(
        id: WidgetId,
        label: impl Into<String>,
        sizing: WidgetSizing,
        message: Message,
    ) -> Self
    where
        Message: Clone + Send + Sync + 'static,
    {
        Self::widget(
            ButtonWidget::new(id, PaintText::from(label.into()), sizing),
            WidgetMessageMapper::button_message(message),
        )
    }

    /// Build a button leaf node with a custom widget-to-host message mapper.
    pub fn button_mapped(
        id: WidgetId,
        label: impl Into<String>,
        sizing: WidgetSizing,
        map: impl Fn(ButtonMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            ButtonWidget::new(id, PaintText::from(label.into()), sizing),
            WidgetMessageMapper::button(map),
        )
    }
}
