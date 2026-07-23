use super::{DEFAULT_UI_SCALE, DpiScale, ThemeTokens, ViewportScaleTier, effective_ui_scale};
use crate::gui::types::Rgba8;

#[test]
fn viewport_width_maps_to_scale_tiers() {
    assert_eq!(
        ViewportScaleTier::from_viewport_width(820.0),
        ViewportScaleTier::Compact
    );
    assert_eq!(
        ViewportScaleTier::from_viewport_width(1280.0),
        ViewportScaleTier::Standard
    );
    assert_eq!(
        ViewportScaleTier::from_viewport_width(2300.0),
        ViewportScaleTier::Wide
    );
}

#[test]
fn effective_ui_scale_clamps_and_applies_default_multiplier() {
    assert_eq!(effective_ui_scale(0.5), DEFAULT_UI_SCALE);
    assert!((effective_ui_scale(1.5) - (1.5 * DEFAULT_UI_SCALE)).abs() < 0.0001);
    assert_eq!(effective_ui_scale(4.0), 3.0);
}

#[test]
fn dpi_scale_sanitizes_and_converts_between_native_and_logical_units() {
    assert_eq!(DpiScale::new(f64::NAN), DpiScale::ONE);
    assert_eq!(DpiScale::new(0.0), DpiScale::ONE);

    let scale = DpiScale::new(1.5);
    assert_eq!(scale.factor(), 1.5);
    assert_eq!(scale.physical_to_logical(150.0), 100.0);
    assert_eq!(scale.logical_to_physical(100.0), 150.0);
}

#[test]
fn dark_theme_uses_one_workspace_background_and_a_distinct_control_overlay() {
    let theme = ThemeTokens::dark();
    assert_ne!(theme.surface_base, theme.surface_overlay);
    assert_eq!(theme.clear_color, theme.bg_primary);
    assert_eq!(theme.bg_primary, theme.bg_secondary);
    assert_eq!(theme.bg_secondary, theme.bg_tertiary);
    assert_eq!(theme.bg_tertiary, theme.surface_base);
    assert_eq!(theme.surface_base, theme.surface_raised);
}

#[test]
fn dark_theme_matches_editorial_terminal_palette() {
    let theme = ThemeTokens::dark();

    assert_eq!(theme.clear_color, Rgba8::new(27, 30, 30, 255));
    assert_eq!(theme.bg_primary, Rgba8::new(27, 30, 30, 255));
    assert_eq!(theme.surface_overlay, Rgba8::new(42, 45, 45, 255));
    assert_eq!(theme.border, Rgba8::new(58, 61, 61, 255));
    assert_eq!(theme.border_emphasis, Rgba8::new(64, 67, 66, 255));
    assert_eq!(theme.grid_strong, Rgba8::new(54, 57, 57, 255));
    assert_eq!(theme.grid_soft, Rgba8::new(40, 43, 43, 255));
    assert_eq!(theme.accent_mint, Rgba8::new(233, 88, 67, 255));
    assert_eq!(theme.text_primary, Rgba8::new(216, 215, 211, 255));
    assert_eq!(theme.text_muted, Rgba8::new(153, 155, 154, 255));
}

#[test]
fn dark_theme_exposes_non_zero_motion_and_state_layers() {
    let theme = ThemeTokens::dark();
    assert!(theme.state_hover_soft > 0.0);
    assert!(theme.state_hover_strong >= theme.state_hover_soft);
    assert!(theme.motion_speed_transport > 0.0);
    assert!(theme.motion_focus_wave_amp > 0.0);
}

#[test]
fn dark_theme_resolves_viewport_tier_motion_without_compatibility_shell() {
    let compact = ThemeTokens::dark_for_viewport_width(820.0);
    let wide = ThemeTokens::dark_for_viewport_width(2300.0);
    assert!(compact.motion_speed_transport < wide.motion_speed_transport);
    assert!(compact.scrim_modal_alpha < wide.scrim_modal_alpha);
}
