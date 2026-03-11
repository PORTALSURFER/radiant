//! Status-bar progress helpers used by the native-shell frame builder.

use super::super::*;

/// Build compact footer copy for active non-modal job progress.
pub(super) fn status_progress_label(model: &AppModel) -> String {
    if model.progress_overlay.cancel_requested {
        return String::from("Cancelling job…");
    }
    match model.progress_overlay.detail.as_deref() {
        Some(detail) if !detail.trim().is_empty() => format!("job active | {detail}"),
        _ if !model.progress_overlay.title.trim().is_empty() => {
            format!("job active | {}", model.progress_overlay.title)
        }
        _ => String::from("job active"),
    }
}

/// Build the compact right-aligned progress counter shown in the footer.
pub(super) fn status_progress_counter(model: &AppModel) -> String {
    if model.progress_overlay.cancel_requested {
        return String::from("cancelling");
    }
    if model.progress_overlay.total == 0 {
        return String::from("active");
    }
    format!(
        "{}/{}",
        model.progress_overlay.completed, model.progress_overlay.total
    )
}

/// Resolve text bounds for the inline footer progress label and counter.
pub(super) fn status_progress_text_rects(segment: Rect, sizing: SizingTokens) -> (Rect, Rect) {
    let text_rect = compute_status_text_line_rect(segment, sizing, sizing.font_status);
    let counter_width = (text_rect.width() * 0.24).clamp(52.0, 84.0);
    let gap = sizing.status_segment_gap.max(6.0);
    let counter_min_x = (text_rect.max.x - counter_width).max(text_rect.min.x);
    let label_max_x = (counter_min_x - gap).max(text_rect.min.x);
    (
        Rect::from_min_max(text_rect.min, Point::new(label_max_x, text_rect.max.y)),
        Rect::from_min_max(Point::new(counter_min_x, text_rect.min.y), text_rect.max),
    )
}

/// Resolve the compact footer progress-track rect inside the status center segment.
pub(super) fn status_progress_track_rect(segment: Rect, sizing: SizingTokens) -> Rect {
    let inset_left = (sizing.text_inset_x + sizing.header_label_gutter).max(0.0);
    let inset_right = sizing.text_inset_x.max(0.0);
    let track_height = sizing.border_width.max(2.0);
    let max_y = (segment.max.y - sizing.text_inset_y.max(1.0)).min(segment.max.y - 1.0);
    let min_y = (max_y - track_height).max(segment.min.y + 1.0);
    Rect::from_min_max(
        Point::new(segment.min.x + inset_left, min_y),
        Point::new(
            (segment.max.x - inset_right).max(segment.min.x + inset_left),
            max_y,
        ),
    )
}

/// Resolve the filled portion of the footer progress track.
pub(super) fn status_progress_fill_rect(track_rect: Rect, model: &AppModel) -> Option<Rect> {
    if track_rect.width() <= 0.0 || track_rect.height() <= 0.0 {
        return None;
    }
    let fraction = if model.progress_overlay.total == 0 {
        0.18
    } else {
        (model.progress_overlay.completed as f32 / model.progress_overlay.total as f32)
            .clamp(0.0, 1.0)
    };
    let fill_width = (track_rect.width() * fraction).max(0.0);
    if fill_width <= 0.0 {
        return None;
    }
    Some(Rect::from_min_max(
        track_rect.min,
        Point::new(
            track_rect.min.x + fill_width.min(track_rect.width()),
            track_rect.max.y,
        ),
    ))
}
