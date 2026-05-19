//! Runtime builder helpers for image primitives.

use crate::gui::types::ImageRgba;
use crate::runtime::SurfaceNode;
use crate::widgets::contract::{WidgetId, WidgetSizing};
use std::sync::Arc;

use super::ImageWidget;

impl<Message> SurfaceNode<Message> {
    /// Build a non-emitting raster image leaf node.
    pub fn image(id: WidgetId, image: Arc<ImageRgba>, sizing: WidgetSizing) -> Self {
        Self::static_widget(ImageWidget::new(id, image, sizing))
    }
}
