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
    /// Root frame inset around the full shell viewport.
    pub frame_inset: f32,
    /// Shared gap between major shell panels.
    pub panel_gap: f32,
    /// Fixed top-bar band height.
    pub top_bar_height: f32,
    /// Fixed status-bar band height.
    pub status_bar_height: f32,
    /// Sidebar width ratio against body width.
    pub sidebar_ratio: f32,
    /// Sidebar minimum width.
    pub sidebar_min_width: f32,
    /// Sidebar maximum width.
    pub sidebar_max_width: f32,
    /// Main content minimum width.
    pub content_min_width: f32,
    /// Waveform card height ratio against content height.
    pub waveform_ratio: f32,
    /// Waveform card minimum height.
    pub waveform_min_height: f32,
    /// Waveform card maximum height.
    pub waveform_max_height: f32,
    /// Gap between triage columns.
    pub column_gap: f32,
    /// Minimum width allowed for each triage column.
    pub column_min_width: f32,
    /// Shared panel inset for nested regions.
    pub panel_inset: f32,
    /// Gap between browser rows.
    pub browser_row_gap: f32,
    /// Browser row card height.
    pub browser_row_height: f32,
    /// Gap between source rows.
    pub source_row_gap: f32,
    /// Source row card height.
    pub source_row_height: f32,
    /// Space between compact metadata text rows.
    pub text_row_gap: f32,
    /// Horizontal text inset inside row cards.
    pub text_inset_x: f32,
    /// Vertical text inset inside row cards.
    pub text_inset_y: f32,
    /// Top block height reserved for source header + search line.
    pub source_header_block_height: f32,
    /// Top block height reserved for triage column headers.
    pub column_header_block_height: f32,
    /// Top block height reserved for waveform title + metadata.
    pub waveform_header_block_height: f32,
    /// Bottom padding reserved for source list footer hints.
    pub source_bottom_padding: f32,
    /// Bottom padding reserved for triage columns.
    pub column_bottom_padding: f32,
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
        Self::for_viewport_width(1280.0)
    }
}

impl StyleTokens {
    /// Build style tokens tuned for a viewport width tier.
    pub(crate) fn for_viewport_width(viewport_width: f32) -> Self {
        let mut tokens = Self {
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
                frame_inset: 7.0,
                panel_gap: 6.0,
                top_bar_height: 36.0,
                status_bar_height: 20.0,
                sidebar_ratio: 0.22,
                sidebar_min_width: 176.0,
                sidebar_max_width: 280.0,
                content_min_width: 220.0,
                waveform_ratio: 0.35,
                waveform_min_height: 126.0,
                waveform_max_height: 250.0,
                column_gap: 6.0,
                column_min_width: 40.0,
                panel_inset: 6.0,
                browser_row_gap: 3.0,
                browser_row_height: 21.0,
                source_row_gap: 3.0,
                source_row_height: 20.0,
                text_row_gap: 2.0,
                text_inset_x: 5.0,
                text_inset_y: 3.0,
                source_header_block_height: 34.0,
                column_header_block_height: 20.0,
                waveform_header_block_height: 30.0,
                source_bottom_padding: 8.0,
                column_bottom_padding: 6.0,
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
        };
        if viewport_width < 980.0 {
            tokens.sizing.frame_inset = 6.0;
            tokens.sizing.panel_gap = 5.0;
            tokens.sizing.top_bar_height = 34.0;
            tokens.sizing.status_bar_height = 20.0;
            tokens.sizing.sidebar_ratio = 0.23;
            tokens.sizing.sidebar_min_width = 168.0;
            tokens.sizing.sidebar_max_width = 252.0;
            tokens.sizing.content_min_width = 180.0;
            tokens.sizing.waveform_ratio = 0.34;
            tokens.sizing.waveform_min_height = 120.0;
            tokens.sizing.waveform_max_height = 220.0;
            tokens.sizing.column_gap = 5.0;
            tokens.sizing.panel_inset = 5.0;
            tokens.sizing.browser_row_gap = 2.0;
            tokens.sizing.browser_row_height = 19.0;
            tokens.sizing.source_row_gap = 2.0;
            tokens.sizing.source_row_height = 18.0;
            tokens.sizing.source_header_block_height = 32.0;
            tokens.sizing.column_header_block_height = 19.0;
            tokens.sizing.waveform_header_block_height = 28.0;
            tokens.sizing.source_bottom_padding = 6.0;
            tokens.sizing.column_bottom_padding = 5.0;
            tokens.sizing.waveform_scan_step = 10.0;
            tokens.sizing.font_title = 13.0;
            tokens.sizing.font_header = 11.0;
            tokens.sizing.font_body = 9.5;
            tokens.sizing.font_meta = 9.5;
            tokens.sizing.font_status = 10.5;
            return tokens;
        }
        if viewport_width > 1700.0 {
            tokens.sizing.frame_inset = 10.0;
            tokens.sizing.panel_gap = 8.0;
            tokens.sizing.top_bar_height = 38.0;
            tokens.sizing.status_bar_height = 22.0;
            tokens.sizing.sidebar_ratio = 0.20;
            tokens.sizing.sidebar_min_width = 190.0;
            tokens.sizing.sidebar_max_width = 320.0;
            tokens.sizing.content_min_width = 260.0;
            tokens.sizing.waveform_ratio = 0.36;
            tokens.sizing.waveform_min_height = 140.0;
            tokens.sizing.waveform_max_height = 280.0;
            tokens.sizing.column_gap = 8.0;
            tokens.sizing.panel_inset = 8.0;
            tokens.sizing.browser_row_gap = 4.0;
            tokens.sizing.browser_row_height = 24.0;
            tokens.sizing.source_row_gap = 4.0;
            tokens.sizing.source_row_height = 23.0;
            tokens.sizing.source_header_block_height = 38.0;
            tokens.sizing.column_header_block_height = 24.0;
            tokens.sizing.waveform_header_block_height = 34.0;
            tokens.sizing.source_bottom_padding = 10.0;
            tokens.sizing.column_bottom_padding = 8.0;
            tokens.sizing.waveform_scan_step = 14.0;
            tokens.sizing.font_title = 15.0;
            tokens.sizing.font_header = 13.0;
            tokens.sizing.font_body = 11.0;
            tokens.sizing.font_meta = 10.5;
            tokens.sizing.font_status = 11.5;
        }
        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::StyleTokens;

    #[test]
    fn viewport_tiers_adjust_row_heights() {
        let narrow = StyleTokens::for_viewport_width(820.0);
        let standard = StyleTokens::for_viewport_width(1280.0);
        let wide = StyleTokens::for_viewport_width(1900.0);
        assert!(narrow.sizing.browser_row_height < standard.sizing.browser_row_height);
        assert!(standard.sizing.browser_row_height < wide.sizing.browser_row_height);
        assert!(narrow.sizing.source_row_height < wide.sizing.source_row_height);
    }

    #[test]
    fn viewport_tiers_adjust_header_bands() {
        let narrow = StyleTokens::for_viewport_width(900.0);
        let wide = StyleTokens::for_viewport_width(1800.0);
        assert!(narrow.sizing.column_header_block_height < wide.sizing.column_header_block_height);
        assert!(
            narrow.sizing.waveform_header_block_height < wide.sizing.waveform_header_block_height
        );
    }

    #[test]
    fn viewport_tiers_adjust_shell_frame_metrics() {
        let narrow = StyleTokens::for_viewport_width(900.0);
        let wide = StyleTokens::for_viewport_width(1800.0);
        assert!(narrow.sizing.top_bar_height < wide.sizing.top_bar_height);
        assert!(narrow.sizing.frame_inset < wide.sizing.frame_inset);
        assert!(narrow.sizing.column_gap < wide.sizing.column_gap);
    }
}
