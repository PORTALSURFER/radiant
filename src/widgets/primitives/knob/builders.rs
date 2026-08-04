//! Runtime builder helpers for knob primitives.

use crate::runtime::{SurfaceNode, WidgetMessageMapper};
use crate::widgets::contract::{WidgetId, WidgetSizing};
use crate::widgets::interaction::{KnobEditBatch, KnobMessage};

use super::{KnobWidget, RetainedKnobWidget};

impl<Message> WidgetMessageMapper<Message> {
    /// Build a mapper for legacy Knob messages and typed lifecycle batches.
    pub fn knob(map: impl Fn(KnobMessage) -> Message + 'static) -> Self {
        Self::dynamic(move |output| {
            if let Some(batch) = output.typed_cloned::<KnobEditBatch>() {
                return batch.legacy_message().map(&map);
            }
            output.typed_cloned::<KnobMessage>().map(&map)
        })
    }

    /// Build a mapper that receives the complete ordered knob edit batch.
    pub fn knob_edits(map: impl Fn(KnobEditBatch) -> Message + 'static) -> Self {
        Self::typed(map)
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a knob leaf that maps legacy lifecycle messages.
    pub fn knob(
        id: WidgetId,
        value: f32,
        sizing: WidgetSizing,
        map: impl Fn(KnobMessage) -> Message + 'static,
    ) -> Self {
        Self::knob_mapped(id, value, sizing, map)
    }

    /// Build a knob leaf that maps legacy lifecycle messages.
    pub fn knob_mapped(
        id: WidgetId,
        value: f32,
        sizing: WidgetSizing,
        map: impl Fn(KnobMessage) -> Message + 'static,
    ) -> Self {
        let mut knob = KnobWidget::new(id, value);
        knob.common.sizing = sizing;
        Self::widget(
            RetainedKnobWidget::new(knob),
            WidgetMessageMapper::knob(map),
        )
    }

    /// Build a knob leaf that forwards complete ordered edit batches.
    pub fn knob_edits_mapped(
        id: WidgetId,
        value: f32,
        sizing: WidgetSizing,
        map: impl Fn(KnobEditBatch) -> Message + 'static,
    ) -> Self {
        let mut knob = KnobWidget::new(id, value);
        knob.common.sizing = sizing;
        Self::widget(
            RetainedKnobWidget::new(knob),
            WidgetMessageMapper::knob_edits(map),
        )
    }
}
