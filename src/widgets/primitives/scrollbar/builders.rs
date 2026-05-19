use crate::{
    runtime::{SurfaceNode, WidgetMessageMapper},
    widgets::{
        contract::{WidgetId, WidgetSizing},
        interaction::ScrollbarMessage,
    },
};

use super::{ScrollbarAxis, ScrollbarWidget};

impl<Message> WidgetMessageMapper<Message> {
    /// Build a scrollbar-message mapper.
    pub fn scrollbar(map: impl Fn(ScrollbarMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a scrollbar leaf that maps offset changes by normalized offset.
    pub fn scrollbar(
        id: WidgetId,
        axis: ScrollbarAxis,
        sizing: WidgetSizing,
        map: impl Fn(f32) -> Message + Send + Sync + 'static,
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
        map: impl Fn(ScrollbarMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            ScrollbarWidget::new(id, axis, sizing),
            WidgetMessageMapper::scrollbar(map),
        )
    }
}
