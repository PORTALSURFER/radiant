use super::{DEFAULT_UI_SCALE, DpiScale, ThemeTokens, ViewportScaleTier, effective_ui_scale};

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
fn dark_theme_uses_distinct_primary_and_overlay_surfaces() {
    let theme = ThemeTokens::dark();
    assert_ne!(theme.surface_base, theme.surface_overlay);
    assert_ne!(theme.bg_primary, theme.bg_tertiary);
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
