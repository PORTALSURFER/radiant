//! Waveform overlay emission for playhead, selection, loop, and scrollbar chrome.

use super::scrollbar::emit_waveform_scrollbar;
use super::surface::emit_waveform_loading_placeholder;
use super::trail::emit_waveform_playhead_trail;
use super::*;

pub(in crate::gui::native_shell::state) fn push_waveform_playhead_overlay(
    primitives: &mut impl PrimitiveSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &NativeMotionModel,
    selection_flash_active: bool,
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
        let selection_fill = if selection_flash_active {
            translucent_overlay_color(style.surface_overlay, style.accent_warning, 0.78)
        } else {
            translucent_overlay_color(style.bg_secondary, style.accent_warning, 0.52)
        };
        let selection_border = if selection_flash_active {
            blend_color(style.accent_warning, style.text_primary, 0.5)
        } else {
            blend_color(style.accent_warning, style.text_primary, 0.28)
        };
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect,
                color: selection_fill,
            }),
        );
        push_border(
            primitives,
            rect,
            selection_border,
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
