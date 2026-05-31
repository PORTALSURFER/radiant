use crate::{
    application::{ViewNode, view_node_from_widget},
    gui::types::Rgba8,
    widgets::{ColorMarkerAlign, ColorMarkerWidget},
};

/// Builder for passive color markers used as swatches and list indicators.
pub struct ColorMarkerBuilder {
    widget: ColorMarkerWidget,
}

impl ColorMarkerBuilder {
    /// Set the preferred marker side length.
    pub fn side(mut self, side: u8) -> Self {
        self.widget = self.widget.with_side(side);
        self
    }

    /// Set the horizontal edge inset.
    pub fn inset(mut self, inset: u8) -> Self {
        self.widget = self.widget.with_inset(inset);
        self
    }

    /// Set the horizontal alignment inside the assigned bounds.
    pub fn align(mut self, align: ColorMarkerAlign) -> Self {
        self.widget = self.widget.with_align(align);
        self
    }

    /// Build this passive marker view.
    pub fn view<Message: 'static>(self) -> ViewNode<Message> {
        view_node_from_widget(self.widget)
    }
}

/// Build a passive color marker with an optional fill color.
pub fn color_marker(color: Option<Rgba8>) -> ColorMarkerBuilder {
    ColorMarkerBuilder {
        widget: ColorMarkerWidget::new(color),
    }
}
