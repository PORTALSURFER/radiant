//! Runtime builder helpers for slider primitives.

use crate::runtime::{SurfaceNode, WidgetMessageMapper};
use crate::widgets::contract::{WidgetId, WidgetSizing};
use crate::widgets::interaction::{SliderEditBatch, SliderMessage};

use super::{SliderWidget, retained::RetainedSliderWidget};

impl<Message> WidgetMessageMapper<Message> {
    /// Build a slider-message mapper.
    pub fn slider(map: impl Fn(SliderMessage) -> Message + 'static) -> Self {
        Self::dynamic(move |output| {
            if let Some(batch) = output.typed_cloned::<SliderEditBatch>() {
                return batch
                    .value_change()
                    .map(|value| map(SliderMessage::ValueChanged { value }));
            }
            output.typed_cloned::<SliderMessage>().map(&map)
        })
    }

    /// Build a mapper that receives the slider's complete ordered edit batch.
    pub fn slider_edits(map: impl Fn(SliderEditBatch) -> Message + 'static) -> Self {
        Self::typed(map)
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a slider leaf that maps value changes by normalized value.
    pub fn slider(
        id: WidgetId,
        value: f32,
        sizing: WidgetSizing,
        map: impl Fn(f32) -> Message + 'static,
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
        map: impl Fn(SliderMessage) -> Message + 'static,
    ) -> Self {
        Self::widget(
            RetainedSliderWidget::new(SliderWidget::new(id, value, sizing)),
            WidgetMessageMapper::slider(map),
        )
    }

    /// Build a slider leaf that forwards the complete ordered edit batch.
    pub fn slider_edits_mapped(
        id: WidgetId,
        value: f32,
        sizing: WidgetSizing,
        map: impl Fn(SliderEditBatch) -> Message + 'static,
    ) -> Self {
        Self::widget(
            RetainedSliderWidget::new(SliderWidget::new(id, value, sizing)),
            WidgetMessageMapper::slider_edits(map),
        )
    }
}
