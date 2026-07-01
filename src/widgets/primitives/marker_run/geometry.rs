use crate::gui::types::{Point, Rect};

use super::model::MarkerRunAlign;

#[derive(Clone, Copy)]
pub(super) struct MarkerRunGeometry {
    side: u8,
    gap: u8,
    inset: u8,
    align: MarkerRunAlign,
}

pub(super) fn marker_geometry(
    side: u8,
    gap: u8,
    inset: u8,
    align: MarkerRunAlign,
) -> MarkerRunGeometry {
    MarkerRunGeometry {
        side,
        gap,
        inset,
        align,
    }
}

pub(super) fn for_each_marker_rect(
    bounds: Rect,
    count: usize,
    geometry: MarkerRunGeometry,
    mut push: impl FnMut(usize, Rect),
) {
    if !bounds.has_finite_positive_area() || count == 0 || geometry.side == 0 {
        return;
    }

    let side = (geometry.side as f32)
        .min(bounds.width())
        .min(bounds.height());
    if side <= 0.0 {
        return;
    }

    let gap = geometry.gap as f32;
    let total_width = count as f32 * side + count.saturating_sub(1) as f32 * gap;
    let start_x = marker_start_x(bounds, geometry.align, total_width, geometry.inset as f32);
    let y = bounds.min.y + (bounds.height() - side) * 0.5;
    for index in 0..count {
        let x = start_x + index as f32 * (side + gap);
        let rect = Rect::from_min_max(Point::new(x, y), Point::new(x + side, y + side));
        if let Some(rect) = clip_marker_rect(rect, bounds) {
            push(index, rect);
        }
    }
}

pub(super) fn collect_marker_rects(
    bounds: Rect,
    count: usize,
    geometry: MarkerRunGeometry,
    output: &mut Vec<Rect>,
) {
    output.clear();
    for_each_marker_rect(bounds, count, geometry, |_, rect| output.push(rect));
}

fn marker_start_x(bounds: Rect, align: MarkerRunAlign, total_width: f32, inset: f32) -> f32 {
    match align {
        MarkerRunAlign::Left => (bounds.min.x + inset).min(bounds.max.x - total_width),
        MarkerRunAlign::Center => bounds.min.x + (bounds.width() - total_width) * 0.5,
        MarkerRunAlign::Right => (bounds.max.x - total_width - inset).max(bounds.min.x),
    }
}

fn clip_marker_rect(rect: Rect, bounds: Rect) -> Option<Rect> {
    let clipped = Rect::from_min_max(
        Point::new(rect.min.x.max(bounds.min.x), rect.min.y.max(bounds.min.y)),
        Point::new(rect.max.x.min(bounds.max.x), rect.max.y.min(bounds.max.y)),
    );
    clipped.has_finite_positive_area().then_some(clipped)
}
