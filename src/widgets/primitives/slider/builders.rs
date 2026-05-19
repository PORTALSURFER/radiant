//! Runtime builder helpers for slider primitives.

use crate::runtime::{SurfaceNode, WidgetMessageMapper};
use crate::widgets::contract::{WidgetId, WidgetSizing};
use crate::widgets::interaction::SliderMessage;

use super::SliderWidget;

impl<Message> WidgetMessageMapper<Message> {
    /// Build a slider-message mapper.
    pub fn slider(map: impl Fn(SliderMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a slider leaf that maps value changes by normalized value.
    pub fn slider(
        id: WidgetId,
        value: f32,
        sizing: WidgetSizing,
        map: impl Fn(f32) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::slider_mapped(id, value, sizing, move |message| match message {
            SliderMessage::ValueChanged { value } => map(value),
        })
    }

    /// Build a slider leaf with a custom widget-to-host message mapper.
    pub fn slider_mapped(
        id: WidgetId,
        value: f32,
        sizing: WidgetSizing,
        map: impl Fn(SliderMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            SliderWidget::new(id, value, sizing),
            WidgetMessageMapper::slider(map),
        )
    }
}
