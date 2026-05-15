use crate::{
    application::{MappedWidget, ViewNode, view_node_from_widget},
    runtime::WidgetMessageMapper,
    widgets::{ScrollbarAxis, ScrollbarMessage, ScrollbarWidget, WidgetSizing, WidgetStyle},
};

/// Builder for application-level scrollbars.
pub struct ScrollbarBuilder {
    axis: ScrollbarAxis,
    viewport_fraction: f32,
    offset_fraction: f32,
    style: Option<WidgetStyle>,
}

impl ScrollbarBuilder {
    /// Apply an explicit widget style before binding this scrollbar.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Set the visible viewport fraction.
    pub fn viewport_fraction(mut self, viewport_fraction: f32) -> Self {
        self.viewport_fraction = viewport_fraction;
        self
    }

    /// Set the current normalized offset fraction.
    pub fn offset_fraction(mut self, offset_fraction: f32) -> Self {
        self.offset_fraction = offset_fraction;
        self
    }

    /// Emit a host message mapped from scrollbar messages.
    pub fn mapped<Message: 'static>(
        self,
        map: impl Fn(ScrollbarMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let size = match self.axis {
            ScrollbarAxis::Horizontal => crate::layout::Vector2::new(120.0, 6.0),
            ScrollbarAxis::Vertical => crate::layout::Vector2::new(6.0, 120.0),
        };
        let mut scrollbar = ScrollbarWidget::new(0, self.axis, WidgetSizing::fixed(size));
        scrollbar.props.viewport_fraction = self.viewport_fraction;
        scrollbar.state.offset_fraction = self.offset_fraction;
        let mut node = view_node_from_widget(MappedWidget::new(
            scrollbar,
            WidgetMessageMapper::scrollbar(map),
        ));
        node.style = self.style;
        node
    }
}

/// Build an application-level scrollbar.
pub fn scrollbar(axis: ScrollbarAxis) -> ScrollbarBuilder {
    ScrollbarBuilder {
        axis,
        viewport_fraction: 1.0,
        offset_fraction: 0.0,
        style: None,
    }
}
