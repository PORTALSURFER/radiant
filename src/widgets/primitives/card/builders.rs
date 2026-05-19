//! Runtime builder helpers for card primitives.

use crate::runtime::SurfaceNode;
use crate::widgets::contract::{WidgetId, WidgetSizing};

use super::CardWidget;

impl<Message> SurfaceNode<Message> {
    /// Build a non-emitting card or panel leaf node.
    pub fn card(id: WidgetId, sizing: WidgetSizing) -> Self {
        Self::static_widget(CardWidget::new(id, sizing))
    }
}
