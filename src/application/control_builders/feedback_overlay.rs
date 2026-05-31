use crate::{
    application::{ViewNode, view_node_from_widget},
    gui::{paint::BorderSides, types::Rgba8},
    widgets::FeedbackOverlayWidget,
};

/// Builder for passive feedback overlays.
pub struct FeedbackOverlayBuilder {
    widget: FeedbackOverlayWidget,
}

impl FeedbackOverlayBuilder {
    /// Paint a full-bounds background tint.
    pub fn background(mut self, color: Rgba8) -> Self {
        self.widget = self.widget.with_background(color);
        self
    }

    /// Paint a determinate progress fill.
    pub fn progress(mut self, fraction: f32, color: Rgba8) -> Self {
        self.widget = self.widget.with_progress(fraction, color);
        self
    }

    /// Paint edge-band accents.
    pub fn edge(mut self, color: Rgba8, thickness: f32, sides: BorderSides) -> Self {
        self.widget = self.widget.with_edge(color, thickness, sides);
        self
    }

    /// Build this passive overlay view.
    pub fn view<Message: 'static>(self) -> ViewNode<Message> {
        view_node_from_widget(self.widget)
    }
}

/// Build an empty passive feedback overlay.
pub fn feedback_overlay() -> FeedbackOverlayBuilder {
    FeedbackOverlayBuilder {
        widget: FeedbackOverlayWidget::fill(),
    }
}
