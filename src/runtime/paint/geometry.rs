use super::{PaintPointList, PaintPrimitive, PaintStrokeRect};
use crate::{
    gui::types::{Point, Rect, Rgba8},
    widgets::WidgetId,
};

pub(crate) fn blend_color(from: Rgba8, to: Rgba8, amount: f32) -> Rgba8 {
    from.blend_toward(to, amount)
}

pub(crate) fn diagonal_cut_rect_points(rect: Rect) -> PaintPointList {
    let cut = (rect.height().min(rect.width()) * 0.18).clamp(4.0, 8.0);
    [
        Point::new(rect.min.x, rect.min.y),
        Point::new(rect.max.x, rect.min.y),
        Point::new(rect.max.x, rect.max.y - cut),
        Point::new(rect.max.x - cut, rect.max.y),
        Point::new(rect.min.x, rect.max.y),
    ]
    .into()
}

pub(crate) fn push_axis_stroke(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    bounds: Rect,
    color: Rgba8,
    horizontal: bool,
) {
    let rect = if horizontal {
        Rect::from_min_max(
            Point::new(bounds.min.x, bounds.min.y + bounds.height() * 0.5),
            Point::new(bounds.max.x, bounds.min.y + bounds.height() * 0.5),
        )
    } else {
        Rect::from_min_max(
            Point::new(bounds.min.x + bounds.width() * 0.5, bounds.min.y),
            Point::new(bounds.min.x + bounds.width() * 0.5, bounds.max.y),
        )
    };
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id,
        rect,
        color,
        width: 1.0,
    }));
}

pub(crate) fn inset_rect(rect: Rect, x: f32, y: f32) -> Rect {
    Rect::from_min_max(
        Point::new(rect.min.x + x, rect.min.y + y),
        Point::new(rect.max.x - x, rect.max.y - y),
    )
}
