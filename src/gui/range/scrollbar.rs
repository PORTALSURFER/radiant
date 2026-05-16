use crate::gui::types::{Point, Rect};

/// Request used to resolve one horizontal normalized-range scrollbar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NormalizedScrollbarRequest {
    /// Scrollbar track in window coordinates.
    pub track: Rect,
    /// Visible normalized start in micro-units (`0..=1_000_000`).
    pub start_micros: u32,
    /// Visible normalized end in micro-units (`0..=1_000_000`).
    pub end_micros: u32,
    /// Minimum thumb width in pixels.
    pub min_thumb_width: f32,
}

/// Resolved horizontal normalized-range scrollbar geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NormalizedScrollbar {
    /// Scrollbar track in window coordinates.
    pub track: Rect,
    /// Scrollbar thumb in window coordinates.
    pub thumb: Rect,
}

/// Resolve horizontal scrollbar geometry for a normalized viewport.
pub fn resolve_normalized_scrollbar(
    request: NormalizedScrollbarRequest,
) -> Option<NormalizedScrollbar> {
    if request.track.width() <= 1.0 || request.track.height() <= 1.0 {
        return None;
    }
    let (start, _, span) = clamped_normalized_span(request.start_micros, request.end_micros);
    if span >= 999_999 {
        return None;
    }

    let min_thumb_width = request.min_thumb_width.max(1.0).min(request.track.width());
    let thumb_width = (request.track.width() * (span as f32 / 1_000_000.0))
        .round()
        .clamp(min_thumb_width, request.track.width());
    let travel = (request.track.width() - thumb_width).max(0.0);
    let max_start = 1_000_000u32.saturating_sub(span);
    let start_ratio = if max_start == 0 {
        0.0
    } else {
        start.min(max_start) as f32 / max_start as f32
    };
    let thumb_min_x = (request.track.min.x + travel * start_ratio).round();
    let thumb_max_x = (thumb_min_x + thumb_width).min(request.track.max.x);
    let thumb = Rect::from_min_max(
        Point::new(thumb_min_x, request.track.min.y),
        Point::new(thumb_max_x.max(thumb_min_x + 1.0), request.track.max.y),
    );
    Some(NormalizedScrollbar {
        track: request.track,
        thumb,
    })
}

/// Return the pointer offset inside the horizontal scrollbar thumb.
pub fn normalized_scrollbar_thumb_offset_at_point(
    scrollbar: NormalizedScrollbar,
    point: Point,
) -> Option<f32> {
    scrollbar
        .thumb
        .contains(point)
        .then_some((point.x - scrollbar.thumb.min.x).clamp(0.0, scrollbar.thumb.width()))
}

/// Return the pointer's normalized grip ratio inside the horizontal scrollbar thumb.
pub fn normalized_scrollbar_thumb_ratio_at_point(
    scrollbar: NormalizedScrollbar,
    point: Point,
) -> Option<f32> {
    scrollbar.thumb.contains(point).then_some({
        let thumb_width = scrollbar.thumb.width().max(1.0);
        ((point.x - scrollbar.thumb.min.x) / thumb_width).clamp(0.0, 1.0)
    })
}

/// Resolve the normalized viewport center for a dragged scrollbar thumb.
pub fn normalized_scrollbar_center_for_pointer(
    scrollbar: NormalizedScrollbar,
    start_micros: u32,
    end_micros: u32,
    pointer_x: f32,
    thumb_pointer_offset_x: f32,
) -> Option<u32> {
    let (_, _, span) = clamped_normalized_span(start_micros, end_micros);
    let max_start = 1_000_000u32.saturating_sub(span);
    let thumb_width = scrollbar
        .thumb
        .width()
        .max(1.0)
        .min(scrollbar.track.width());
    let travel = (scrollbar.track.width() - thumb_width).max(0.0);
    if travel <= f32::EPSILON || max_start == 0 {
        return Some(500_000);
    }
    let thumb_min_x = (pointer_x - thumb_pointer_offset_x)
        .clamp(scrollbar.track.min.x, scrollbar.track.max.x - thumb_width);
    let start_ratio = ((thumb_min_x - scrollbar.track.min.x) / travel).clamp(0.0, 1.0);
    let view_start = ((start_ratio * max_start as f32).round() as u32).min(max_start);
    Some((view_start + span / 2).min(1_000_000))
}

/// Resolve the normalized viewport center for a click inside the scrollbar track.
pub fn normalized_scrollbar_center_at_point(
    scrollbar: NormalizedScrollbar,
    start_micros: u32,
    end_micros: u32,
    point: Point,
) -> Option<u32> {
    if !scrollbar.track.contains(point) || scrollbar.thumb.contains(point) {
        return None;
    }
    normalized_scrollbar_center_for_pointer(
        scrollbar,
        start_micros,
        end_micros,
        point.x,
        scrollbar.thumb.width() * 0.5,
    )
}

fn clamped_normalized_span(start_micros: u32, end_micros: u32) -> (u32, u32, u32) {
    let start = start_micros.min(1_000_000);
    let end = end_micros.min(1_000_000).max(start.saturating_add(1));
    let span = end.saturating_sub(start).max(1);
    (start, end, span)
}
