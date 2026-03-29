use super::*;

/// Emit preview overlays for detected silence-split waveform slices.
pub(in crate::gui::native_shell::state) fn emit_waveform_slice_previews(
    primitives: &mut impl PrimitiveSink,
    waveform_plot: Rect,
    style: &StyleTokens,
    model: &NativeMotionModel,
) {
    let slice_blue = Rgba8 {
        r: 86,
        g: 156,
        b: 255,
        a: 255,
    };
    let export_amber = Rgba8 {
        r: 255,
        g: 188,
        b: 92,
        a: 255,
    };
    let cleanup_red = Rgba8 {
        r: 255,
        g: 110,
        b: 104,
        a: 255,
    };
    let keep_green = Rgba8 {
        r: 120,
        g: 214,
        b: 146,
        a: 255,
    };
    let slices = compute_waveform_slice_preview_rects(
        waveform_plot,
        &model.waveform_slices,
        model.waveform_view_start_micros,
        model.waveform_view_end_micros,
    );
    for slice in slices {
        let (fill, border) = if slice.duplicate_cleanup_exempted {
            (
                translucent_overlay_color(style.surface_overlay, keep_green, 0.74),
                blend_color(keep_green, style.text_primary, 0.42),
            )
        } else if slice.duplicate_cleanup_candidate {
            if slice.focused {
                (
                    translucent_overlay_color(style.surface_overlay, cleanup_red, 0.82),
                    blend_color(cleanup_red, style.text_primary, 0.55),
                )
            } else {
                (
                    translucent_overlay_color(style.surface_overlay, cleanup_red, 0.62),
                    blend_color(cleanup_red, style.text_primary, 0.34),
                )
            }
        } else if slice.focused {
            (
                translucent_overlay_color(style.surface_overlay, slice_blue, 0.82),
                blend_color(slice_blue, style.text_primary, 0.55),
            )
        } else if slice.marked_for_export {
            (
                translucent_overlay_color(style.surface_overlay, export_amber, 0.68),
                blend_color(export_amber, style.text_primary, 0.42),
            )
        } else if slice.selected {
            (
                translucent_overlay_color(style.surface_overlay, slice_blue, 0.72),
                blend_color(slice_blue, style.text_primary, 0.36),
            )
        } else {
            (
                translucent_overlay_color(style.bg_secondary, slice_blue, 0.44),
                blend_color(slice_blue, style.text_primary, 0.18),
            )
        };
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: slice.rect,
                color: fill,
            }),
        );
        push_border(primitives, slice.rect, border, style.sizing.border_width);
    }
}
