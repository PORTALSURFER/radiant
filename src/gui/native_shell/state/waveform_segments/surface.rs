//! Waveform surface helpers for loading placeholders and rendered images.

use super::*;

pub(in crate::gui::native_shell::state) fn emit_waveform_loading_placeholder(
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

pub(in crate::gui::native_shell::state) fn push_waveform_image(
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
