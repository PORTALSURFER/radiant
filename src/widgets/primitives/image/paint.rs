//! Image paint command generation.

use crate::gui::types::Rect;
use crate::runtime::{PaintImage, PaintPrimitive};
use crate::widgets::primitives::image::ImageWidget;
use std::sync::Arc;

pub(super) fn push_image_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    image: &ImageWidget,
    bounds: Rect,
) {
    primitives.push(PaintPrimitive::Image(PaintImage {
        widget_id: image.common.id,
        source_rect: None,
        rect: bounds,
        image: Arc::clone(&image.props.image),
    }));
}
