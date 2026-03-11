//! Status-bar progress helpers used by the native-shell frame builder.

use super::super::*;

pub(super) fn render_status_bar(
    state: &NativeShellState,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let sizing = style.sizing;
    let status_text = if model.status_text.is_empty() {
        if state.transport_running {
            format!(
                "Transport: running | Selected column: {}",
                state.selected_column + 1
            )
        } else {
            format!(
                "Transport: stopped | Selected column: {}",
                state.selected_column + 1
            )
        }
    } else {
        model.status_text.clone()
    };
    let browser_summary = format!(
        "rows: {} | selected: {} | anchor: {} | search: {}{}",
        model.browser.visible_count,
        model.browser.selected_path_count,
        model
            .browser
            .anchor_visible_row
            .map(|row| row.to_string())
            .unwrap_or_else(|| String::from("—")),
        if model.browser.search_query.is_empty() {
            "—"
        } else {
            model.browser.search_query.as_str()
        },
        if model.browser.busy {
            " | filtering…"
        } else {
            ""
        }
    );
    let status_left = if model.status.left.is_empty() {
        status_text
    } else {
        model.status.left.clone()
    };
    let status_center = if model.status.center.is_empty() {
        browser_summary
    } else {
        model.status.center.clone()
    };
    let status_right = if model.status.right.is_empty() {
        format!("col: {}/3", state.selected_column + 1)
    } else {
        model.status.right.clone()
    };
    let inline_progress_active = model.progress_overlay.visible && !model.progress_overlay.modal;
    let status_left_text_rect =
        compute_status_text_line_rect(layout.status_left_segment, sizing, sizing.font_status);
    let status_center_text_rect =
        compute_status_text_line_rect(layout.status_center_segment, sizing, sizing.font_status);
    let status_right_text_rect = status_right_text_rect(layout.status_right_segment, sizing, None);
    emit_text(
        text_runs,
        TextRun {
            text: truncate_to_width(
                &status_left,
                status_left_text_rect.width().max(36.0),
                sizing.font_status,
            ),
            position: status_left_text_rect.min,
            font_size: sizing.font_status,
            color: style.text_muted,
            max_width: Some(status_left_text_rect.width().max(36.0)),
            align: TextAlign::Left,
        },
    );
    if inline_progress_active {
        let (progress_label_rect, progress_counter_rect) =
            status_progress_text_rects(layout.status_center_segment, sizing);
        let progress_track_rect = status_progress_track_rect(layout.status_center_segment, sizing);
        let progress_label = status_progress_label(model);
        let progress_counter = status_progress_counter(model);
        emit_text(
            text_runs,
            TextRun {
                text: truncate_to_width(
                    &progress_label,
                    progress_label_rect.width().max(36.0),
                    sizing.font_status,
                ),
                position: progress_label_rect.min,
                font_size: sizing.font_status,
                color: style.text_primary,
                max_width: Some(progress_label_rect.width().max(36.0)),
                align: TextAlign::Left,
            },
        );
        emit_text(
            text_runs,
            TextRun {
                text: truncate_to_width(
                    &progress_counter,
                    progress_counter_rect.width().max(24.0),
                    sizing.font_status,
                ),
                position: progress_counter_rect.min,
                font_size: sizing.font_status,
                color: style.text_muted,
                max_width: Some(progress_counter_rect.width().max(24.0)),
                align: TextAlign::Right,
            },
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: progress_track_rect,
                color: style.grid_soft,
            }),
        );
        if let Some(fill_rect) = status_progress_fill_rect(progress_track_rect, model) {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: fill_rect,
                    color: style.accent_mint,
                }),
            );
        }
    } else {
        emit_text(
            text_runs,
            TextRun {
                text: truncate_to_width(
                    &status_center,
                    status_center_text_rect.width().max(36.0),
                    sizing.font_status,
                ),
                position: status_center_text_rect.min,
                font_size: sizing.font_status,
                color: style.text_primary,
                max_width: Some(status_center_text_rect.width().max(36.0)),
                align: TextAlign::Center,
            },
        );
    }
    emit_text(
        text_runs,
        TextRun {
            text: truncate_to_width(
                &status_right,
                status_right_text_rect.width().max(36.0),
                sizing.font_status,
            ),
            position: status_right_text_rect.min,
            font_size: sizing.font_status,
            color: style.text_muted,
            max_width: Some(status_right_text_rect.width().max(36.0)),
            align: TextAlign::Right,
        },
    );
}

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
