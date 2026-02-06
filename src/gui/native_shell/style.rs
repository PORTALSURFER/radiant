//! Shared style tokens for the native shell renderer.

use crate::gui::types::Rgba8;

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}

/// Style tokens consumed by the retained shell paint pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StyleTokens {
    /// Root clear color for the frame.
    pub clear_color: Rgba8,
    /// Primary surface fill.
    pub bg_primary: Rgba8,
    /// Secondary surface fill.
    pub bg_secondary: Rgba8,
    /// Tertiary/raised surface fill.
    pub bg_tertiary: Rgba8,
    /// Standard border color.
    pub border: Rgba8,
    /// Primary grid line color.
    pub grid_strong: Rgba8,
    /// Secondary grid line color.
    pub grid_soft: Rgba8,
    /// Primary selection accent.
    pub accent_mint: Rgba8,
    /// Secondary accent.
    pub accent_copper: Rgba8,
    /// Warning/hover accent.
    pub accent_warning: Rgba8,
    /// High-contrast text color.
    pub text_primary: Rgba8,
    /// Secondary muted text color.
    pub text_muted: Rgba8,
    /// Compact sizing tokens for layout rhythm and element scale.
    pub sizing: SizingTokens,
}

/// Compact sizing tokens used by the native shell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SizingTokens {
    /// Shared panel inset for nested regions.
    pub panel_inset: f32,
    /// Gap between browser rows.
    pub browser_row_gap: f32,
    /// Gap between source rows.
    pub source_row_gap: f32,
    /// Space between compact metadata text rows.
    pub text_row_gap: f32,
    /// Horizontal text inset inside row cards.
    pub text_inset_x: f32,
    /// Vertical text inset inside row cards.
    pub text_inset_y: f32,
    /// Top block height reserved for source header + search line.
    pub source_header_block_height: f32,
    /// Bottom padding reserved for source list footer hints.
    pub source_bottom_padding: f32,
    /// Border stroke width.
    pub border_width: f32,
    /// Waveform scanline step width.
    pub waveform_scan_step: f32,
    /// Primary title font size.
    pub font_title: f32,
    /// Section header font size.
    pub font_header: f32,
    /// Body label font size.
    pub font_body: f32,
    /// Metadata font size.
    pub font_meta: f32,
    /// Status bar font size.
    pub font_status: f32,
    /// Base transport indicator radius.
    pub lamp_radius_base: f32,
    /// Additional transport indicator pulse amplitude.
    pub lamp_radius_amp: f32,
}

impl Default for StyleTokens {
    fn default() -> Self {
        Self {
            clear_color: rgba(12, 11, 10, 255),
            bg_primary: rgba(12, 11, 10, 255),
            bg_secondary: rgba(20, 18, 16, 255),
            bg_tertiary: rgba(28, 26, 23, 255),
            border: rgba(44, 40, 36, 255),
            grid_strong: rgba(55, 50, 45, 255),
            grid_soft: rgba(42, 38, 34, 255),
            accent_mint: rgba(152, 172, 158, 255),
            accent_copper: rgba(186, 148, 108, 255),
            accent_warning: rgba(194, 158, 108, 255),
            text_primary: rgba(224, 227, 234, 255),
            text_muted: rgba(166, 173, 184, 255),
            sizing: SizingTokens {
                panel_inset: 6.0,
                browser_row_gap: 3.0,
                source_row_gap: 3.0,
                text_row_gap: 2.0,
                text_inset_x: 5.0,
                text_inset_y: 3.0,
                source_header_block_height: 34.0,
                source_bottom_padding: 8.0,
                border_width: 1.0,
                waveform_scan_step: 12.0,
                font_title: 14.0,
                font_header: 12.0,
                font_body: 10.0,
                font_meta: 10.0,
                font_status: 11.0,
                lamp_radius_base: 4.0,
                lamp_radius_amp: 2.0,
            },
        }
    }
}
