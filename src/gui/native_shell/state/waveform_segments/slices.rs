use super::*;

/// Emit preview overlays for detected silence-split waveform slices.
pub(in crate::gui::native_shell::state) fn emit_waveform_slice_previews(
    primitives: &mut impl PrimitiveSink,
    waveform_plot: Rect,
    style: &StyleTokens,
    model: &NativeMotionModel,
) {
    let slices = compute_waveform_slice_preview_rects(
        waveform_plot,
        &model.waveform_slices,
        model.waveform_view_start_micros,
        model.waveform_view_end_micros,
    );
    for slice in slices {
        let (fill, border) = if slice.selected {
            (
                translucent_overlay_color(style.surface_overlay, style.highlight_blue, 0.54),
                blend_color(style.highlight_blue, style.text_primary, 0.42),
            )
        } else {
            (
                translucent_overlay_color(style.bg_secondary, style.highlight_blue_soft, 0.26),
                blend_color(style.border_emphasis, style.highlight_blue, 0.20),
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
