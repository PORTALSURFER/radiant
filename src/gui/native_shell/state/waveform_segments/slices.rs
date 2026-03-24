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
    let slices = compute_waveform_slice_preview_rects(
        waveform_plot,
        &model.waveform_slices,
        model.waveform_view_start_micros,
        model.waveform_view_end_micros,
    );
    for slice in slices {
        let (fill, border) = if slice.selected {
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
