use super::wrap01;
use radiant::{
    gui::{paint::PaintFrame, types::Rgba8},
    layout::{Point, Rect},
};

pub(super) fn push_wrapped_ratio_bar(
    frame: &mut PaintFrame,
    lane: Rect,
    center_ratio: f32,
    width_ratio: f32,
    height_ratio: f32,
    color: Rgba8,
) {
    let width = width_ratio.max(0.0);
    let width = width.min(0.08);
    if width <= 0.0 {
        return;
    }
    let center = wrap01(center_ratio);
    let start = center - width * 0.5;
    let end = center + width * 0.5;
    if start < 0.0 {
        push_ratio_bar_segment(frame, lane, start + 1.0, 1.0, height_ratio, color);
        push_ratio_bar_segment(frame, lane, 0.0, end, height_ratio, color);
        return;
    }
    if end > 1.0 {
        push_ratio_bar_segment(frame, lane, start, 1.0, height_ratio, color);
        push_ratio_bar_segment(frame, lane, 0.0, end - 1.0, height_ratio, color);
        return;
    }
    push_ratio_bar_segment(frame, lane, start, end, height_ratio, color);
}

fn push_ratio_bar_segment(
    frame: &mut PaintFrame,
    lane: Rect,
    start: f32,
    end: f32,
    height_ratio: f32,
    color: Rgba8,
) {
    let start = start.clamp(0.0, 1.0);
    let end = end.clamp(0.0, 1.0);
    let height = lane.height() * height_ratio.clamp(0.0, 1.0);
    if end <= start || height <= 0.0 {
        return;
    }
    let center_y = (lane.min.y + lane.max.y) * 0.5;
    frame.push_rect(
        Rect::from_min_max(
            Point::new(lane.x_for_ratio(start), center_y - height * 0.5),
            Point::new(lane.x_for_ratio(end), center_y + height * 0.5),
        ),
        color,
    );
}

pub(super) fn push_ratio_circle(
    frame: &mut PaintFrame,
    track: Rect,
    center_ratio: f32,
    center_y: f32,
    radius: f32,
    color: Rgba8,
) {
    if radius <= 0.0 {
        return;
    }
    let center_x = track.x_for_ratio(center_ratio);
    frame.push_circle(Point::new(center_x, center_y), radius, color);
}

pub(super) fn push_ratio_rect(
    frame: &mut PaintFrame,
    track: Rect,
    start_ratio: f32,
    width_ratio: f32,
    color: Rgba8,
) {
    let start = start_ratio.clamp(0.0, 1.0);
    let end = (start + width_ratio.max(0.0)).clamp(0.0, 1.0);
    if end <= start {
        return;
    }
    frame.push_rect(
        Rect::from_min_max(
            Point::new(track.x_for_ratio(start), track.min.y),
            Point::new(track.x_for_ratio(end), track.max.y),
        ),
        color,
    );
}

pub(super) fn inset(rect: Rect, x: f32, y: f32) -> Rect {
    Rect::from_min_max(
        Point::new(rect.min.x + x, rect.min.y + y),
        Point::new(rect.max.x - x, rect.max.y - y),
    )
}
