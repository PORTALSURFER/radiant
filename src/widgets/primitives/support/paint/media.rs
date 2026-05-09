use crate::gui::types::Rect;
use crate::runtime::{PaintCustomSurface, PaintImage, PaintPrimitive};
use crate::widgets::primitives::{canvas::CanvasWidget, image::ImageWidget};
use std::sync::Arc;

pub(in crate::widgets::primitives) fn push_image_widget_paint(
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

pub(in crate::widgets::primitives) fn push_canvas_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    canvas: &CanvasWidget,
    bounds: Rect,
) {
    primitives.push(PaintPrimitive::CustomSurface(PaintCustomSurface {
        widget_id: canvas.common.id,
        rect: bounds,
        bounds: canvas.common.paint.bounds,
        retained: canvas.retained,
    }));
}
