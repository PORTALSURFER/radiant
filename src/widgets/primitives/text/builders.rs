//! Runtime builder helpers for text primitives.

use crate::runtime::{PaintText, SurfaceNode};
use crate::widgets::contract::{WidgetId, WidgetSizing};

use super::TextWidget;

impl<Message> SurfaceNode<Message> {
    /// Build a non-emitting text leaf node.
    pub fn text(id: WidgetId, text: impl Into<PaintText>, sizing: WidgetSizing) -> Self {
        Self::static_widget(TextWidget::new(id, text, sizing))
    }
}
