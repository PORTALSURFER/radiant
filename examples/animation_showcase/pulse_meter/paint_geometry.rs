use radiant::{
    gui::{
        feedback::{horizontal_value_range_rect, horizontal_wrapped_value_range_rects},
        paint::PaintFrame,
        types::Rgba8,
    },
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
    for segment in horizontal_wrapped_value_range_rects(
        lane,
        center_ratio,
        width_ratio.min(0.08),
        height_ratio,
    )
    .into_iter()
    .flatten()
    {
        frame.push_rect(segment, color);
    }
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
    if let Some(rect) =
        horizontal_value_range_rect(track, start_ratio, start_ratio + width_ratio, 1.0)
    {
        frame.push_rect(rect, color);
    }
}
