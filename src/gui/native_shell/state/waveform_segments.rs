//! Static-frame segment routing and waveform overlay emit helpers.

use super::browser_rows::format_milli_value;
use super::*;

/// Width in logical pixels for edit-fade drag handles.
const EDIT_FADE_HANDLE_WIDTH: f32 = 3.0;
/// Height in logical pixels for loop-range marker bars.
const LOOP_BAR_HEIGHT: f32 = 3.0;

/// One retained ghost line for the dynamic playhead trail.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PlayheadTrailLine {
    /// Normalized x-position in `0.0..=1.0`.
    pub ratio: f32,
    /// Blend amount for the ghost line.
    pub alpha: f32,
}

/// Resolve which static segment owns one primitive.
pub(super) fn static_segment_for_primitive(
    layout: &ShellLayout,
    model: &AppModel,
    primitive: &Primitive,
) -> StaticFrameSegment {
    let anchor = match primitive {
        Primitive::Rect(fill) => rect_center(fill.rect),
        Primitive::Circle(fill) => fill.center,
        Primitive::Image(image) => rect_center(image.rect),
    };
    static_segment_for_point(layout, model, anchor)
}

/// Resolve which static segment owns one text run.
pub(super) fn static_segment_for_text(
    layout: &ShellLayout,
    model: &AppModel,
    text_run: &TextRun,
) -> StaticFrameSegment {
    static_segment_for_point(layout, model, text_run.position)
}

/// Resolve the owning static segment for a point in shell coordinates.
pub(super) fn static_segment_for_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> StaticFrameSegment {
    if layout.status_bar.contains(point) {
        return StaticFrameSegment::StatusBar;
    }
    if layout.waveform_card.contains(point) {
        return StaticFrameSegment::WaveformOverlay;
    }
    if model.map.active
        && (layout.browser_rows.contains(point) || layout.browser_table_header.contains(point))
    {
        return StaticFrameSegment::MapPanel;
    }
    if layout.browser_rows.contains(point) {
        return StaticFrameSegment::BrowserRowsWindow;
    }
    if layout.browser_panel.contains(point)
        || layout.browser_tabs.contains(point)
        || layout.browser_toolbar.contains(point)
        || layout.browser_table_header.contains(point)
        || layout.browser_footer.contains(point)
    {
        return StaticFrameSegment::BrowserFrame;
    }
    StaticFrameSegment::GlobalStatic
}

/// Return whether one static build pass should include the requested segment.
pub(super) fn static_segment_matches(
    filter: Option<StaticFrameSegment>,
    segment: StaticFrameSegment,
) -> bool {
    filter.is_none_or(|target| target == segment)
}

/// Return the geometric center for a rectangle.
pub(super) fn rect_center(rect: Rect) -> Point {
    Point::new(
        rect.min.x + (rect.width() * 0.5),
        rect.min.y + (rect.height() * 0.5),
    )
}

pub(super) fn push_waveform_playhead_overlay(
    primitives: &mut impl PrimitiveSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &NativeMotionModel,
    playhead_trail_lines: &[PlayheadTrailLine],
) {
    let edit_selection_blue = Rgba8 {
        r: 86,
        g: 156,
        b: 255,
        a: 255,
    };
    let annotations = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        model.waveform_selection_milli,
        model.waveform_cursor_milli,
        None,
    );
    let playhead_rect =
        playhead_marker_rect(layout.waveform_plot, style.sizing.border_width, model);

    if let Some(rect) = annotations.selection {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect,
                color: translucent_overlay_color(style.bg_secondary, style.accent_warning, 0.52),
            }),
        );
        push_border(
            primitives,
            rect,
            blend_color(style.accent_warning, style.text_primary, 0.28),
            style.sizing.border_width,
        );
        if model.waveform_loop_enabled {
            emit_waveform_loop_bar(primitives, style, rect);
        }
    }

    if let Some(edit_selection) = model.waveform_edit_selection_milli {
        let edit_selection_rect = compute_waveform_annotation_rects(
            layout.waveform_plot,
            style.sizing.border_width,
            Some(edit_selection),
            None,
            None,
        )
        .selection;
        if let Some(rect) = edit_selection_rect {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect,
                    color: translucent_overlay_color(style.bg_secondary, edit_selection_blue, 0.5),
                }),
            );
            push_border(
                primitives,
                rect,
                blend_color(edit_selection_blue, style.text_primary, 0.24),
                style.sizing.border_width,
            );
            emit_edit_fade_overlays(
                primitives,
                style,
                rect,
                edit_selection,
                model.waveform_edit_fade_in_end_milli,
                model.waveform_edit_fade_out_start_milli,
                edit_selection_blue,
            );
        }
    }

    if let Some(rect) = annotations.cursor {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect,
                color: style.accent_warning,
            }),
        );
    }
    if let Some(rect) = playhead_rect {
        emit_waveform_playhead_trail(
            primitives,
            layout.waveform_plot,
            style,
            style.sizing.border_width,
            playhead_trail_lines,
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect,
                color: style.accent_copper,
            }),
        );
    }
}

/// Resolve the active playhead marker rectangle, preferring high-precision micros.
fn playhead_marker_rect(
    waveform_plot: Rect,
    border_width: f32,
    model: &NativeMotionModel,
) -> Option<Rect> {
    if let Some(playhead_micros) = model.waveform_playhead_micros {
        let ratio = (playhead_micros as f32 / 1_000_000.0).clamp(0.0, 1.0);
        return marker_rect_for_ratio(waveform_plot, border_width, ratio);
    }
    model.waveform_playhead_milli.and_then(|playhead_milli| {
        marker_rect_for_ratio(
            waveform_plot,
            border_width,
            (f32::from(playhead_milli) / 1000.0).clamp(0.0, 1.0),
        )
    })
}

/// Resolve one marker rectangle for a normalized ratio in `0..=1`.
fn marker_rect_for_ratio(waveform_plot: Rect, border_width: f32, ratio: f32) -> Option<Rect> {
    if waveform_plot.width() <= 0.0 || waveform_plot.height() <= 0.0 {
        return None;
    }
    let marker_width = border_width.max(1.0).min(waveform_plot.width());
    let raw_x = waveform_plot.min.x + (waveform_plot.width() * ratio.clamp(0.0, 1.0));
    let left = raw_x.clamp(waveform_plot.min.x, waveform_plot.max.x - marker_width);
    let right = (left + marker_width).min(waveform_plot.max.x);
    Some(Rect::from_min_max(
        Point::new(left, waveform_plot.min.y),
        Point::new(right, waveform_plot.max.y),
    ))
}

/// Emit top/bottom loop-range bars over the active playback selection.
fn emit_waveform_loop_bar(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    selection: Rect,
) {
    let bar_height = LOOP_BAR_HEIGHT
        .max(style.sizing.border_width)
        .min(selection.height().max(1.0));
    let top = Rect::from_min_max(
        selection.min,
        Point::new(
            selection.max.x,
            (selection.min.y + bar_height).min(selection.max.y),
        ),
    );
    let bottom = Rect::from_min_max(
        Point::new(
            selection.min.x,
            (selection.max.y - bar_height).max(selection.min.y),
        ),
        selection.max,
    );
    let edge_color = blend_color(style.accent_copper, style.text_primary, 0.2);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: top,
            color: translucent_overlay_color(style.surface_overlay, style.accent_copper, 0.42),
        }),
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: bottom,
            color: translucent_overlay_color(style.surface_overlay, style.accent_copper, 0.32),
        }),
    );
    push_border(primitives, top, edge_color, style.sizing.border_width);
    push_border(primitives, bottom, edge_color, style.sizing.border_width);
}

/// Emit edit-fade shading and draggable handle markers for the active edit selection.
fn emit_edit_fade_overlays(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    edit_selection_rect: Rect,
    edit_selection: crate::app::NormalizedRangeModel,
    fade_in_end_milli: Option<u16>,
    fade_out_start_milli: Option<u16>,
    accent_blue: Rgba8,
) {
    let selection_start = edit_selection.start_milli.min(edit_selection.end_milli);
    let selection_end = edit_selection.start_milli.max(edit_selection.end_milli);
    if selection_end <= selection_start {
        return;
    }
    let fade_in_end = fade_in_end_milli
        .unwrap_or(selection_start)
        .clamp(selection_start, selection_end);
    let fade_out_start = fade_out_start_milli
        .unwrap_or(selection_end)
        .clamp(selection_start, selection_end);

    let selection_width = edit_selection_rect.width();
    if selection_width <= 0.0 {
        return;
    }

    let x_for_milli = |milli: u16| {
        edit_selection_rect.min.x
            + (selection_width
                * (f32::from(milli.saturating_sub(selection_start))
                    / f32::from((selection_end - selection_start).max(1))))
    };
    let fade_in_x =
        x_for_milli(fade_in_end).clamp(edit_selection_rect.min.x, edit_selection_rect.max.x);
    let fade_out_x =
        x_for_milli(fade_out_start).clamp(edit_selection_rect.min.x, edit_selection_rect.max.x);

    if fade_in_x > edit_selection_rect.min.x {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(
                    Point::new(edit_selection_rect.min.x, edit_selection_rect.min.y),
                    Point::new(fade_in_x, edit_selection_rect.max.y),
                ),
                color: translucent_overlay_color(style.surface_overlay, accent_blue, 0.22),
            }),
        );
    }
    if fade_out_x < edit_selection_rect.max.x {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(
                    Point::new(fade_out_x, edit_selection_rect.min.y),
                    Point::new(edit_selection_rect.max.x, edit_selection_rect.max.y),
                ),
                color: translucent_overlay_color(style.surface_overlay, accent_blue, 0.22),
            }),
        );
    }

    emit_edit_fade_handle(
        primitives,
        style,
        edit_selection_rect,
        fade_in_x,
        accent_blue,
    );
    emit_edit_fade_handle(
        primitives,
        style,
        edit_selection_rect,
        fade_out_x,
        accent_blue,
    );
}

/// Emit one draggable edit-fade handle marker.
fn emit_edit_fade_handle(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    edit_selection_rect: Rect,
    x: f32,
    accent_blue: Rgba8,
) {
    let width = EDIT_FADE_HANDLE_WIDTH
        .max(style.sizing.border_width)
        .max(1.0);
    let half = width * 0.5;
    let left = (x - half).clamp(edit_selection_rect.min.x, edit_selection_rect.max.x - 1.0);
    let right = (left + width)
        .min(edit_selection_rect.max.x)
        .max(left + 1.0);
    let handle = Rect::from_min_max(
        Point::new(left, edit_selection_rect.min.y),
        Point::new(right, edit_selection_rect.max.y),
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: handle,
            color: translucent_overlay_color(style.bg_secondary, accent_blue, 0.62),
        }),
    );
    push_border(
        primitives,
        handle,
        blend_color(accent_blue, style.text_primary, 0.42),
        style.sizing.border_width,
    );
}

/// Emit retained ghost lines behind the active playhead.
fn emit_waveform_playhead_trail(
    primitives: &mut impl PrimitiveSink,
    waveform_plot: Rect,
    style: &StyleTokens,
    border_width: f32,
    trail_lines: &[PlayheadTrailLine],
) {
    for line in trail_lines {
        let Some(rect) = marker_rect_for_ratio(waveform_plot, border_width, line.ratio) else {
            continue;
        };
        let amount = line.alpha.clamp(0.0, 1.0);
        if amount <= 0.0 {
            continue;
        }
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect,
                color: translucent_overlay_color(
                    style.surface_overlay,
                    style.accent_copper,
                    amount,
                ),
            }),
        );
    }
}

pub(super) fn push_waveform_image(
    primitives: &mut impl PrimitiveSink,
    waveform_plot: Rect,
    image: Option<&ImageRgba>,
) {
    let Some(image) = image else {
        return;
    };
    if image.width == 0
        || image.height == 0
        || waveform_plot.width() <= 0.0
        || waveform_plot.height() <= 0.0
    {
        return;
    }

    let has_visible_pixels = image
        .pixels
        .chunks_exact(4)
        .any(|pixel| pixel.get(3).copied().unwrap_or(0) > 0);
    if !has_visible_pixels {
        return;
    }
    emit_primitive(
        primitives,
        Primitive::Image(DrawImage {
            rect: waveform_plot,
            image: std::sync::Arc::new(image.clone()),
        }),
    );
}

pub(super) fn push_waveform_header_overlay(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &NativeMotionModel,
    toolbar_left: Option<f32>,
) {
    let sizing = style.sizing;
    let text_layout = compute_waveform_header_text_layout(layout.waveform_header, sizing);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: layout.waveform_header,
            color: style.surface_raised,
        }),
    );
    let content_right = toolbar_left
        .unwrap_or(layout.waveform_header.max.x - sizing.text_inset_x)
        .clamp(
            text_layout.title_row.min.x + 24.0,
            layout.waveform_header.max.x,
        );
    let title_max_width = text_layout
        .title_row
        .width()
        .min((content_right - text_layout.title_row.min.x).max(24.0))
        .max(24.0);
    emit_text(
        text_runs,
        TextRun {
            text: truncate_to_width(
                model.waveform_loaded_label.as_deref().unwrap_or("Waveform"),
                title_max_width,
                sizing.font_header,
            ),
            position: text_layout.title_row.min,
            font_size: sizing.font_header,
            color: style.text_primary,
            max_width: Some(title_max_width),
            align: TextAlign::Left,
        },
    );
    let playhead_text = model
        .waveform_playhead_milli
        .map(format_milli_value)
        .unwrap_or_else(|| String::from("—"));
    let cursor_text = model
        .waveform_cursor_milli
        .map(format_milli_value)
        .unwrap_or_else(|| String::from("—"));
    let view_text = format!(
        "{}..{}",
        format_milli_value(model.waveform_view_start_milli),
        format_milli_value(model.waveform_view_end_milli)
    );
    let tempo_text = model.waveform_tempo_label.as_deref().unwrap_or("— BPM");
    let zoom_text = model.waveform_zoom_label.as_deref().unwrap_or("100%");
    let metadata_max_width = text_layout
        .metadata_row
        .width()
        .min((content_right - text_layout.metadata_row.min.x).max(24.0))
        .max(24.0);
    emit_text(
        text_runs,
        TextRun {
            text: format!(
                "{} | tempo: {} | zoom: {} | playhead: {} | cursor: {} | view: {}",
                model.waveform_transport_hint,
                tempo_text,
                zoom_text,
                playhead_text,
                cursor_text,
                view_text,
            ),
            position: text_layout.metadata_row.min,
            font_size: sizing.font_meta,
            color: style.text_muted,
            max_width: Some(metadata_max_width),
            align: TextAlign::Left,
        },
    );
}
