//! Shared style tokens for the native shell renderer.

use crate::gui::types::Rgba8;

/// Semantic color tokens used by the retained shell paint pass.
mod palette;
/// Geometry and typography sizing tokens plus UI-scale inflation rules.
mod sizing;
/// Viewport tier selection and non-geometric tier policy values.
mod tier;

pub(crate) use sizing::SizingTokens;
pub(crate) use tier::LayoutScaleTier;

/// Style tokens consumed by the retained shell paint pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StyleTokens {
    /// Viewport scale tier used to derive the token set.
    pub layout_tier: LayoutScaleTier,
    /// Root clear color for the frame.
    pub clear_color: Rgba8,
    /// Primary surface fill.
    pub bg_primary: Rgba8,
    /// Secondary surface fill.
    pub bg_secondary: Rgba8,
    /// Tertiary/raised surface fill.
    pub bg_tertiary: Rgba8,
    /// Base shell surface for content-heavy regions.
    pub surface_base: Rgba8,
    /// Raised shell surface for cards/chrome regions.
    pub surface_raised: Rgba8,
    /// Overlay/dialog surface fill.
    pub surface_overlay: Rgba8,
    /// Standard border color.
    pub border: Rgba8,
    /// Emphasized border color for clustered chrome boundaries.
    pub border_emphasis: Rgba8,
    /// Divider color used between source-management sidebar sections.
    pub source_section_divider: Rgba8,
    /// Recovery badge fill color when entries are present but idle.
    pub source_recovery_badge_idle: Rgba8,
    /// Recovery badge fill color while recovery is actively running.
    pub source_recovery_badge_active: Rgba8,
    /// Disabled control fill color for action buttons.
    pub control_disabled_fill: Rgba8,
    /// Primary grid line color.
    pub grid_strong: Rgba8,
    /// Secondary grid line color.
    pub grid_soft: Rgba8,
    /// Primary selection accent.
    pub accent_mint: Rgba8,
    /// Secondary accent.
    pub accent_copper: Rgba8,
    /// Negative/trash accent used for destructive triage indicators.
    pub accent_trash: Rgba8,
    /// Warning/hover accent.
    pub accent_warning: Rgba8,
    /// Vibrant orange highlight used for strong warning emphasis.
    pub highlight_orange: Rgba8,
    /// Softer orange highlight used for secondary warning emphasis.
    pub highlight_orange_soft: Rgba8,
    /// Vibrant cobalt-blue highlight used for transport and loop accents.
    pub highlight_blue: Rgba8,
    /// Softer azure-blue highlight used for secondary focus feedback.
    pub highlight_blue_soft: Rgba8,
    /// Vibrant cyan highlight used for positive active emphasis.
    pub highlight_cyan: Rgba8,
    /// Softer aqua-cyan highlight used for secondary positive emphasis.
    pub highlight_cyan_soft: Rgba8,
    /// High-contrast text color.
    pub text_primary: Rgba8,
    /// Secondary muted text color.
    pub text_muted: Rgba8,
    /// Blend amount for subtle hover states.
    pub state_hover_soft: f32,
    /// Blend amount for stronger hover states.
    pub state_hover_strong: f32,
    /// Blend amount for selected-state fills.
    pub state_selected_blend: f32,
    /// Blend amount for pulsing focused-state fills/borders.
    pub state_focus_pulse_blend: f32,
    /// Pulse speed used while transport is running.
    pub motion_speed_transport: f32,
    /// Pulse speed used while transport is stopped but focus emphasis is active.
    pub motion_speed_idle: f32,
    /// Additional blend amplitude injected into focused row/card fills.
    pub motion_focus_wave_amp: f32,
    /// Additional blend amplitude injected into focused text emphasis.
    pub motion_focus_text_wave_amp: f32,
    /// Alpha used by non-modal background scrims.
    pub scrim_soft_alpha: u8,
    /// Alpha used by modal-blocking background scrims.
    pub scrim_modal_alpha: u8,
    /// Compact sizing tokens for layout rhythm and element scale.
    pub sizing: SizingTokens,
}

impl Default for StyleTokens {
    fn default() -> Self {
        Self::for_viewport_width(1280.0)
    }
}

impl StyleTokens {
    /// Build style tokens tuned for a viewport width and DPI scale factor.
    ///
    /// The input scale factor is clamped to a safe range so accidental outlier
    /// values cannot collapse or overinflate layout geometry. A small baseline
    /// multiplier is then applied to keep the default UI density slightly larger.
    pub(crate) fn for_viewport_with_scale(viewport_width: f32, ui_scale: f32) -> Self {
        let mut tokens = Self::for_tier(LayoutScaleTier::from_viewport_width(viewport_width));
        tokens.sizing = tokens
            .sizing
            .with_ui_scale(tier::effective_ui_scale(ui_scale));
        tokens
    }

    /// Build style tokens tuned for a viewport width tier.
    pub(crate) fn for_viewport_width(viewport_width: f32) -> Self {
        Self::for_viewport_with_scale(viewport_width, 1.0)
    }

    /// Build style tokens for an explicit scale tier.
    pub(crate) fn for_tier(layout_tier: LayoutScaleTier) -> Self {
        let palette = palette::palette_for_tier(layout_tier);
        let motion = tier::visual_policy_for_tier(layout_tier);
        Self {
            layout_tier,
            clear_color: palette.clear_color,
            bg_primary: palette.bg_primary,
            bg_secondary: palette.bg_secondary,
            bg_tertiary: palette.bg_tertiary,
            surface_base: palette.surface_base,
            surface_raised: palette.surface_raised,
            surface_overlay: palette.surface_overlay,
            border: palette.border,
            border_emphasis: palette.border_emphasis,
            source_section_divider: palette.source_section_divider,
            source_recovery_badge_idle: palette.source_recovery_badge_idle,
            source_recovery_badge_active: palette.source_recovery_badge_active,
            control_disabled_fill: palette.control_disabled_fill,
            grid_strong: palette.grid_strong,
            grid_soft: palette.grid_soft,
            accent_mint: palette.accent_mint,
            accent_copper: palette.accent_copper,
            accent_trash: palette.accent_trash,
            accent_warning: palette.accent_warning,
            highlight_orange: palette.highlight_orange,
            highlight_orange_soft: palette.highlight_orange_soft,
            highlight_blue: palette.highlight_blue,
            highlight_blue_soft: palette.highlight_blue_soft,
            highlight_cyan: palette.highlight_cyan,
            highlight_cyan_soft: palette.highlight_cyan_soft,
            text_primary: palette.text_primary,
            text_muted: palette.text_muted,
            state_hover_soft: motion.state_hover_soft,
            state_hover_strong: motion.state_hover_strong,
            state_selected_blend: motion.state_selected_blend,
            state_focus_pulse_blend: motion.state_focus_pulse_blend,
            motion_speed_transport: motion.motion_speed_transport,
            motion_speed_idle: motion.motion_speed_idle,
            motion_focus_wave_amp: motion.motion_focus_wave_amp,
            motion_focus_text_wave_amp: motion.motion_focus_text_wave_amp,
            scrim_soft_alpha: motion.scrim_soft_alpha,
            scrim_modal_alpha: motion.scrim_modal_alpha,
            sizing: sizing::sizing_for_tier(layout_tier),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LayoutScaleTier, StyleTokens, tier::DEFAULT_UI_SCALE};

    #[test]
    fn viewport_width_maps_to_expected_tier() {
        assert_eq!(
            LayoutScaleTier::from_viewport_width(820.0),
            LayoutScaleTier::Compact
        );
        assert_eq!(
            LayoutScaleTier::from_viewport_width(1280.0),
            LayoutScaleTier::Standard
        );
        assert_eq!(
            LayoutScaleTier::from_viewport_width(2300.0),
            LayoutScaleTier::Wide
        );
    }

    #[test]
    fn explicit_tier_builder_matches_width_builder() {
        let compact = StyleTokens::for_tier(LayoutScaleTier::Compact).sizing;
        let compact_from_width = StyleTokens::for_viewport_width(820.0).sizing;
        assert_eq!(compact.with_ui_scale(DEFAULT_UI_SCALE), compact_from_width);

        let standard = StyleTokens::for_tier(LayoutScaleTier::Standard).sizing;
        let standard_from_width = StyleTokens::for_viewport_width(1280.0).sizing;
        assert_eq!(
            standard.with_ui_scale(DEFAULT_UI_SCALE),
            standard_from_width
        );

        let wide = StyleTokens::for_tier(LayoutScaleTier::Wide).sizing;
        let wide_from_width = StyleTokens::for_viewport_width(2300.0).sizing;
        assert_eq!(wide.with_ui_scale(DEFAULT_UI_SCALE), wide_from_width);
    }

    #[test]
    fn viewport_tiers_adjust_row_heights() {
        let narrow = StyleTokens::for_viewport_width(820.0);
        let standard = StyleTokens::for_viewport_width(1280.0);
        let wide = StyleTokens::for_viewport_width(2300.0);
        assert!(narrow.sizing.browser_row_height < standard.sizing.browser_row_height);
        assert!(standard.sizing.browser_row_height < wide.sizing.browser_row_height);
        assert!(narrow.sizing.source_row_height < wide.sizing.source_row_height);
    }

    #[test]
    fn viewport_tiers_adjust_header_bands() {
        let narrow = StyleTokens::for_viewport_width(900.0);
        let wide = StyleTokens::for_viewport_width(2200.0);
        assert!(narrow.sizing.column_header_block_height < wide.sizing.column_header_block_height);
        assert!(
            narrow.sizing.waveform_header_block_height < wide.sizing.waveform_header_block_height
        );
    }

    #[test]
    fn viewport_tiers_adjust_shell_frame_metrics() {
        let compact = StyleTokens::for_viewport_width(900.0);
        let standard = StyleTokens::for_viewport_width(1280.0);
        let wide = StyleTokens::for_viewport_width(2200.0);
        assert!(wide.sizing.top_bar_height >= standard.sizing.top_bar_height);
        assert!(wide.sizing.frame_inset >= standard.sizing.frame_inset);
        assert!(wide.sizing.column_gap >= standard.sizing.column_gap);
        assert!(compact.sizing.top_bar_height >= standard.sizing.top_bar_height);
    }

    #[test]
    fn viewport_tiers_adjust_render_row_caps() {
        let narrow = StyleTokens::for_viewport_width(900.0);
        let wide = StyleTokens::for_viewport_width(2200.0);
        assert!(narrow.sizing.source_rows_max < wide.sizing.source_rows_max);
        assert!(narrow.sizing.folder_rows_max < wide.sizing.folder_rows_max);
        assert!(
            narrow.sizing.browser_rows_max_per_column < wide.sizing.browser_rows_max_per_column
        );
    }

    #[test]
    fn standard_tier_matches_classic_dense_shell_targets() {
        let standard = StyleTokens::for_viewport_width(1280.0);
        assert!((0.14..=0.18).contains(&standard.sizing.sidebar_ratio));
        assert!(
            (15.5 * DEFAULT_UI_SCALE..=17.0 * DEFAULT_UI_SCALE)
                .contains(&standard.sizing.browser_row_height)
        );
        assert!(standard.sizing.browser_tabs_height <= 20.0 * DEFAULT_UI_SCALE);
        assert!(standard.sizing.browser_toolbar_height <= 21.0 * DEFAULT_UI_SCALE);
        assert!(standard.sizing.browser_table_header_height <= 20.0 * DEFAULT_UI_SCALE);
        assert!(standard.sizing.waveform_ratio <= 0.36);
        assert!(standard.sizing.sidebar_max_width <= 220.0 * DEFAULT_UI_SCALE);
        assert!(standard.sizing.font_body <= 9.1 * DEFAULT_UI_SCALE);
        assert!(standard.sizing.font_meta <= 8.8 * DEFAULT_UI_SCALE);
    }

    #[test]
    fn viewport_scale_preserves_tiers_and_inflates_geometry() {
        let scaled = StyleTokens::for_viewport_with_scale(1280.0, 1.5);
        let base = StyleTokens::for_viewport_width(1280.0);
        assert_eq!(scaled.layout_tier, base.layout_tier);
        assert_eq!(scaled.sizing.sidebar_ratio, base.sizing.sidebar_ratio);
        assert!((scaled.sizing.font_body - (base.sizing.font_body * 1.5)).abs() < 0.0001);
        assert!((scaled.sizing.top_bar_height - (base.sizing.top_bar_height * 1.5)).abs() < 0.0001);
        assert!(
            (scaled.sizing.action_button_width - (base.sizing.action_button_width * 1.5)).abs()
                < 0.0001
        );
        assert!(
            (scaled.sizing.sidebar_max_width - (base.sizing.sidebar_max_width * 1.5)).abs()
                < 0.0001
        );
    }

    #[test]
    fn viewport_scale_is_clamped() {
        let below = StyleTokens::for_viewport_with_scale(1280.0, 0.5);
        let identity = StyleTokens::for_viewport_with_scale(1280.0, 1.0);
        let above = StyleTokens::for_viewport_with_scale(1280.0, 4.0);
        let max = StyleTokens::for_viewport_with_scale(1280.0, 3.0);
        assert_eq!(below, identity);
        assert!((above.sizing.font_body - max.sizing.font_body).abs() < 0.0001);
    }

    #[test]
    fn viewport_tiers_adjust_interaction_density_tokens() {
        let narrow = StyleTokens::for_viewport_width(900.0);
        let wide = StyleTokens::for_viewport_width(2200.0);
        assert!(narrow.sizing.focus_stroke_width < wide.sizing.focus_stroke_width);
        assert!(narrow.sizing.header_to_rows_gap < wide.sizing.header_to_rows_gap);
        assert!(narrow.sizing.row_corner_inset < wide.sizing.row_corner_inset);
        assert!(narrow.sizing.header_label_gutter < wide.sizing.header_label_gutter);
        assert!(narrow.sizing.status_segment_gap < wide.sizing.status_segment_gap);
        assert!(narrow.sizing.recovery_badge_height < wide.sizing.recovery_badge_height);
        assert!(narrow.sizing.recovery_badge_min_width < wide.sizing.recovery_badge_min_width);
        assert!(narrow.state_hover_strong < wide.state_hover_strong);
        assert!(narrow.motion_speed_transport < wide.motion_speed_transport);
        assert!(narrow.motion_focus_wave_amp < wide.motion_focus_wave_amp);
    }
}
