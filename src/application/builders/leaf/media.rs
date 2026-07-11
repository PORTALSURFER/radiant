use super::super::defaults::default_canvas_sizing;
use super::core::view_node_from_widget;
use crate::{
    application::ViewNode,
    gui::types::ImageRgba,
    layout::Vector2,
    widgets::{CanvasWidget, ImageWidget, WidgetSizing},
};
use std::sync::Arc;

/// Build a passive canvas view for retained surfaces that need a generic paint
/// or input slot without host messages.
pub fn canvas<Message: 'static>() -> ViewNode<Message> {
    view_node_from_widget(CanvasWidget::new(0, default_canvas_sizing()))
}

/// Build a non-interactive raster image view.
pub fn image<Message: 'static>(image: Arc<ImageRgba>) -> ViewNode<Message> {
    let size = Vector2::new(image.width().max(1) as f32, image.height().max(1) as f32);
    view_node_from_widget(ImageWidget::new(0, image, WidgetSizing::fixed(size)))
}
