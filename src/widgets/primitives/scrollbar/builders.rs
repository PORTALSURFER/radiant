use crate::{
    runtime::{SurfaceNode, WidgetMessageMapper},
    widgets::{
        contract::{WidgetId, WidgetSizing},
        interaction::{ScrollbarEditBatch, ScrollbarMessage},
    },
};

use super::{RetainedScrollbarWidget, ScrollbarAxis, ScrollbarWidget};

impl<Message> WidgetMessageMapper<Message> {
    /// Build a scrollbar-message mapper.
    pub fn scrollbar(map: impl Fn(ScrollbarMessage) -> Message + 'static) -> Self {
        Self::dynamic(move |output| {
            if let Some(batch) = output.typed_cloned::<ScrollbarEditBatch>() {
                return batch.offset_change().map(|offset_fraction| {
                    map(ScrollbarMessage::OffsetChanged { offset_fraction })
                });
            }
            output.typed_cloned::<ScrollbarMessage>().map(&map)
        })
    }
}

impl<Message> WidgetMessageMapper<Message> {
    /// Map the complete ordered scrollbar edit batch.
    pub fn scrollbar_edits(map: impl Fn(ScrollbarEditBatch) -> Message + 'static) -> Self {
        Self::typed(map)
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a scrollbar leaf forwarding its complete edit lifecycle.
    pub fn scrollbar_edits_mapped(
        id: WidgetId,
        axis: ScrollbarAxis,
        viewport_fraction: f32,
        offset_fraction: f32,
        sizing: WidgetSizing,
        map: impl Fn(ScrollbarEditBatch) -> Message + 'static,
    ) -> Self {
        let mut scrollbar = ScrollbarWidget::new(id, axis, sizing);
        scrollbar.props.viewport_fraction = viewport_fraction;
        scrollbar.state.offset_fraction = offset_fraction;
        Self::widget(
            RetainedScrollbarWidget::new(scrollbar),
            WidgetMessageMapper::scrollbar_edits(map),
        )
    }

    /// Build a scrollbar leaf that maps offset changes by normalized offset.
    pub fn scrollbar(
        id: WidgetId,
        axis: ScrollbarAxis,
        sizing: WidgetSizing,
        map: impl Fn(f32) -> Message + 'static,
    ) -> Self {
        Self::scrollbar_mapped(id, axis, sizing, move |message| match message {
            ScrollbarMessage::OffsetChanged { offset_fraction } => map(offset_fraction),
        })
    }

    /// Build a scrollbar leaf with a custom widget-to-host message mapper.
    pub fn scrollbar_mapped(
        id: WidgetId,
        axis: ScrollbarAxis,
        sizing: WidgetSizing,
        map: impl Fn(ScrollbarMessage) -> Message + 'static,
    ) -> Self {
        Self::widget(
            RetainedScrollbarWidget::new(ScrollbarWidget::new(id, axis, sizing)),
            WidgetMessageMapper::scrollbar(map),
        )
    }
}
