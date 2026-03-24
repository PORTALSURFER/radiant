//! Visual fill, pulse, and meter helpers shared by native shell lists and toolbars.

use super::*;

pub(in crate::gui::native_shell::state) fn format_milli_value(value: u16) -> String {
    format!("{:.3}", f32::from(value.min(1000)) / 1000.0)
}

pub(in crate::gui::native_shell::state) fn volume_action_for_meter(
    volume_meter: Rect,
    point: Point,
) -> UiAction {
    let width = volume_meter.width().max(1.0);
    let clamped_x = point.x.clamp(volume_meter.min.x, volume_meter.max.x);
    let ratio = ((clamped_x - volume_meter.min.x) / width).clamp(0.0, 1.0);
    UiAction::SetVolume {
        value_milli: (ratio * 1000.0).round() as u16,
    }
}

pub(in crate::gui::native_shell::state) fn interaction_wave(pulse_phase: f32) -> f32 {
    ((pulse_phase.sin() + 1.0) * 0.5).clamp(0.0, 1.0)
}

pub(in crate::gui::native_shell::state) fn translucent_overlay_color(
    base: Rgba8,
    tint: Rgba8,
    amount: f32,
) -> Rgba8 {
    let amount = amount.clamp(0.0, 1.0);
    let mut color = blend_color(base, tint, amount);
    color.a = (amount * (base.a as f32 / 255.0) * (tint.a as f32 / 255.0) * 255.0)
        .round()
        .clamp(0.0, 255.0) as u8;
    color
}

/// Return a subtle whitish row-hover fill used across non-browser item lists.
pub(in crate::gui::native_shell::state) fn subtle_item_hover_fill(style: &StyleTokens) -> Rgba8 {
    translucent_overlay_color(
        style.bg_tertiary,
        style.text_primary,
        (style.state_hover_soft * 0.95).clamp(0.12, 0.26),
    )
}

/// Return the stronger hover fill used for sample-browser rows.
///
/// The browser hover needs to read clearly against alternating row fills, so it
/// intentionally uses roughly double the shared item-list hover intensity.
pub(in crate::gui::native_shell::state) fn browser_row_hover_fill(style: &StyleTokens) -> Rgba8 {
    translucent_overlay_color(
        style.bg_tertiary,
        style.text_primary,
        (style.state_hover_soft * 1.9).clamp(0.24, 0.52),
    )
}

/// Return the alternating neutral fill used for non-selected browser rows.
pub(in crate::gui::native_shell::state) fn browser_row_stripe_fill(
    style: &StyleTokens,
    visible_row: usize,
) -> Rgba8 {
    if visible_row % 2 == 0 {
        blend_color(style.surface_base, style.bg_tertiary, 0.14)
    } else {
        blend_color(style.surface_base, style.bg_secondary, 0.10)
    }
}

/// Return the stronger neutral fill used for selected browser rows.
pub(in crate::gui::native_shell::state) fn selected_browser_row_fill(style: &StyleTokens) -> Rgba8 {
    translucent_overlay_color(
        style.bg_tertiary,
        style.text_primary,
        (style.state_selected_blend + 0.14).clamp(0.22, 0.30),
    )
}

pub(in crate::gui::native_shell::state) fn blend_color(a: Rgba8, b: Rgba8, amount: f32) -> Rgba8 {
    let amount = amount.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| -> u8 {
        ((x as f32) + ((y as f32 - x as f32) * amount))
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Rgba8 {
        r: mix(a.r, b.r),
        g: mix(a.g, b.g),
        b: mix(a.b, b.b),
        a: mix(a.a, b.a),
    }
}
