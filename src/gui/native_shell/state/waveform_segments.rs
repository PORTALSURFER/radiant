//! Static-frame segment routing and waveform overlay emit helpers.

use super::browser_rows::format_milli_value;
use super::*;

/// Resolve which static segment owns one primitive.
pub(super) fn static_segment_for_primitive(
    layout: &ShellLayout,
    model: &AppModel,
    primitive: &Primitive,
) -> StaticFrameSegment {
    let anchor = match primitive {
        Primitive::Rect(fill) => rect_center(fill.rect),
        Primitive::Circle(fill) => fill.center,
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
) {
    let annotations = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        model.waveform_selection_milli,
        model.waveform_cursor_milli,
        model.waveform_playhead_milli,
    );

    if let Some(rect) = annotations.selection {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect,
                color: style.grid_strong,
            }),
        );
        push_border(
            primitives,
            rect,
            style.accent_mint,
            style.sizing.border_width,
        );
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
    if let Some(rect) = annotations.playhead {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect,
                color: style.accent_copper,
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

    let plot_width = waveform_plot.width();
    let plot_height = waveform_plot.height();
    let src_width = image.width as f32;
    let src_height = image.height as f32;
    let stride = image.width.saturating_mul(4);

    for x in 0..image.width {
        let x0 = waveform_plot.min.x + (x as f32 * plot_width) / src_width;
        let x1 = waveform_plot.min.x + ((x + 1) as f32 * plot_width) / src_width;
        let mut y = 0usize;
        while y < image.height {
            let idx = y * stride + x * 4;
            if image.pixels[idx + 3] == 0 {
                y += 1;
                continue;
            }
            let y0 = y;
            let red = image.pixels[idx];
            let green = image.pixels[idx + 1];
            let blue = image.pixels[idx + 2];
            let alpha = image.pixels[idx + 3];

            let mut y1 = y0 + 1;
            while y1 < image.height {
                let span_idx = y1 * stride + x * 4;
                if image.pixels[span_idx + 3] == 0
                    || image.pixels[span_idx] != red
                    || image.pixels[span_idx + 1] != green
                    || image.pixels[span_idx + 2] != blue
                    || image.pixels[span_idx + 3] != alpha
                {
                    break;
                }
                y1 += 1;
            }

            let top = waveform_plot.min.y + (y0 as f32 / src_height) * plot_height;
            let bottom = waveform_plot.min.y + (y1 as f32 / src_height) * plot_height;
            if bottom > top {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: Rect::from_min_max(
                            Point::new(x0, top),
                            Point::new(
                                x1.min(waveform_plot.max.x),
                                bottom.min(waveform_plot.max.y),
                            ),
                        ),
                        color: Rgba8 {
                            r: red,
                            g: green,
                            b: blue,
                            a: alpha,
                        },
                    }),
                );
            }
            y = y1;
        }
    }
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
