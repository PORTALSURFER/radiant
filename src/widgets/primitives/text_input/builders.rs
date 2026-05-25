//! Runtime builder helpers for text-input primitives.

use crate::runtime::{SurfaceNode, WidgetMessageMapper};
use crate::widgets::contract::{WidgetId, WidgetSizing};
use crate::widgets::interaction::TextInputMessage;

use super::TextInputWidget;

impl<Message> WidgetMessageMapper<Message> {
    /// Build a text-input-message mapper.
    pub fn text_input(map: impl Fn(TextInputMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a single-line text input that maps edits and submissions by value.
    pub fn text_input(
        id: WidgetId,
        value: impl Into<String>,
        sizing: WidgetSizing,
        map: impl Fn(String) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::text_input_mapped(id, value, sizing, move |message| match message {
            TextInputMessage::Changed { value }
            | TextInputMessage::Submitted { value }
            | TextInputMessage::CompletionRequested { value } => map(value),
        })
    }

    /// Build a single-line text input with a custom widget-to-host message mapper.
    pub fn text_input_mapped(
        id: WidgetId,
        value: impl Into<String>,
        sizing: WidgetSizing,
        map: impl Fn(TextInputMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            TextInputWidget::new(id, value, sizing),
            WidgetMessageMapper::text_input(map),
        )
    }
}
