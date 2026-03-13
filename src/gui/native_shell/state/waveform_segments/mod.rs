//! Static-frame segment routing and waveform overlay emit helpers.

use super::browser_rows::format_milli_value;
use super::*;

mod fades;
mod scrollbar;
mod selection;
mod trail;

use self::{
    fades::emit_edit_fade_overlays,
    scrollbar::emit_waveform_scrollbar,
    selection::{
        emit_hovered_edit_resize_edge, emit_hovered_selection_resize_edge,
        emit_selection_drag_handle, emit_selection_shift_handle, emit_waveform_loop_bar,
    },
    trail::emit_waveform_playhead_trail,
};
pub(super) use self::{
    scrollbar::{waveform_scrollbar_center_for_pointer, waveform_scrollbar_layout},
    trail::{PlayheadTrailLine, playhead_marker_rect},
};

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
    motion_wave: f32,
    playhead_trail_lines: &[PlayheadTrailLine],
    hovered_resize_edge: Option<WaveformResizeHoverEdge>,
) {
    if model.waveform_loading {
        emit_waveform_loading_placeholder(primitives, layout.waveform_plot, style, motion_wave);
        return;
    }
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
        model.waveform_view_start_micros,
        model.waveform_view_end_micros,
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
        emit_hovered_selection_resize_edge(
            primitives,
            style,
            rect,
            style.accent_warning,
            hovered_resize_edge,
        );
        if model.waveform_loop_enabled {
            emit_waveform_loop_bar(primitives, style, rect);
        }
        emit_selection_shift_handle(primitives, style, rect, style.accent_warning);
        emit_selection_drag_handle(primitives, style, rect);
    }

    if let Some(edit_selection) = model.waveform_edit_selection_milli {
        let edit_selection_rect = compute_waveform_annotation_rects(
            layout.waveform_plot,
            style.sizing.border_width,
            Some(edit_selection),
            None,
            None,
            model.waveform_view_start_micros,
            model.waveform_view_end_micros,
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
                layout.waveform_plot,
                rect,
                edit_selection,
                model.waveform_edit_fade_in_end_milli,
                model.waveform_edit_fade_in_end_micros,
                model.waveform_edit_fade_in_mute_start_milli,
                model.waveform_edit_fade_in_mute_start_micros,
                model.waveform_edit_fade_in_curve_milli,
                model.waveform_edit_fade_out_start_milli,
                model.waveform_edit_fade_out_start_micros,
                model.waveform_edit_fade_out_mute_end_milli,
                model.waveform_edit_fade_out_mute_end_micros,
                model.waveform_edit_fade_out_curve_milli,
                model.waveform_view_start_micros,
                model.waveform_view_end_micros,
                edit_selection_blue,
            );
            emit_hovered_edit_resize_edge(
                primitives,
                style,
                rect,
                edit_selection_blue,
                hovered_resize_edge,
            );
            emit_selection_shift_handle(primitives, style, rect, edit_selection_blue);
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
            model.waveform_view_start_micros,
            model.waveform_view_end_micros,
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect,
                color: style.accent_copper,
            }),
        );
    }
    emit_waveform_scrollbar(primitives, layout.waveform_scrollbar_lane, style, model);
}

fn emit_waveform_loading_placeholder(
    primitives: &mut impl PrimitiveSink,
    waveform_plot: Rect,
    style: &StyleTokens,
    motion_wave: f32,
) {
    if waveform_plot.width() <= 0.0 || waveform_plot.height() <= 0.0 {
        return;
    }

    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: waveform_plot,
            color: style.surface_base,
        }),
    );

    let pulse = 0.18 + (motion_wave * 0.12);
    let rail_color = blend_color(style.surface_overlay, style.border_emphasis, 0.22 + pulse);
    let glow_color = blend_color(style.surface_overlay, style.accent_warning, 0.16 + pulse);
    let bar_width = (waveform_plot.width() * 0.14).clamp(12.0, waveform_plot.width() * 0.22);
    let gap = (waveform_plot.width() * 0.05).clamp(10.0, 28.0);
    let total_width = bar_width * 3.0 + gap * 2.0;
    let left = waveform_plot.min.x + ((waveform_plot.width() - total_width) * 0.5).max(0.0);
    let heights = [0.28_f32, 0.52, 0.38];

    for (index, height_ratio) in heights.into_iter().enumerate() {
        let height = (waveform_plot.height() * height_ratio).clamp(18.0, waveform_plot.height());
        let top = waveform_plot.min.y + (waveform_plot.height() - height) * 0.5;
        let min_x = left + index as f32 * (bar_width + gap);
        let bar = Rect::from_min_max(
            Point::new(min_x, top),
            Point::new((min_x + bar_width).min(waveform_plot.max.x), top + height),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: bar,
                color: rail_color,
            }),
        );
        let glow_inset = (bar.width() * 0.18).min(6.0);
        let glow = Rect::from_min_max(
            Point::new(bar.min.x + glow_inset, bar.min.y),
            Point::new(bar.max.x - glow_inset, bar.max.y),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: glow,
                color: glow_color,
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
