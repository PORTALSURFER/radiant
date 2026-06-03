use crate::{
    application::{ViewNode, view_node_from_widget},
    gui::types::Rgba8,
    widgets::{ColorMarkerRunWidget, MarkerRunWidget},
};

/// Builder for passive repeated markers used in ratings, meters, and legends.
pub struct MarkerRunBuilder {
    widget: MarkerRunWidget,
}

/// Builder for passive per-color markers used in legends and status columns.
pub struct ColorMarkerRunBuilder {
    widget: ColorMarkerRunWidget,
}

impl MarkerRunBuilder {
    /// Set each marker side length.
    pub fn side(mut self, side: u8) -> Self {
        self.widget = self.widget.with_side(side);
        self
    }

    /// Set the gap between adjacent markers.
    pub fn gap(mut self, gap: u8) -> Self {
        self.widget = self.widget.with_gap(gap);
        self
    }

    /// Set the horizontal edge inset.
    pub fn inset(mut self, inset: u8) -> Self {
        self.widget = self.widget.with_inset(inset);
        self
    }

    /// Build this passive marker-run view.
    pub fn view<Message: 'static>(self) -> ViewNode<Message> {
        view_node_from_widget(self.widget)
    }
}

impl ColorMarkerRunBuilder {
    /// Set each marker side length.
    pub fn side(mut self, side: u8) -> Self {
        self.widget = self.widget.with_side(side);
        self
    }

    /// Set the gap between adjacent markers.
    pub fn gap(mut self, gap: u8) -> Self {
        self.widget = self.widget.with_gap(gap);
        self
    }

    /// Set the horizontal edge inset.
    pub fn inset(mut self, inset: u8) -> Self {
        self.widget = self.widget.with_inset(inset);
        self
    }

    /// Build this passive marker-run view.
    pub fn view<Message: 'static>(self) -> ViewNode<Message> {
        view_node_from_widget(self.widget)
    }
}

/// Build a passive run of repeated markers.
pub fn marker_run(color: Option<Rgba8>, count: u8) -> MarkerRunBuilder {
    MarkerRunBuilder {
        widget: MarkerRunWidget::new(color, count),
    }
}

/// Build a passive run with one marker per color.
pub fn marker_run_colors(colors: impl IntoIterator<Item = Rgba8>) -> ColorMarkerRunBuilder {
    ColorMarkerRunBuilder {
        widget: ColorMarkerRunWidget::new(colors.into_iter().collect()),
    }
}
