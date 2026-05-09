use crate::gui::types::Rect;
use crate::runtime::{PaintCustomSurface, PaintPrimitive};
use crate::widgets::primitives::canvas::CanvasWidget;

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
