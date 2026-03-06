//! Shared style tokens for the native shell renderer.

use crate::gui::types::Rgba8;

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}

/// Global baseline size multiplier used to keep default shell density slightly larger.
const DEFAULT_UI_SCALE: f32 = 1.06;

const fn clamp_ui_scale(scale: f32) -> f32 {
    scale.clamp(1.0, 3.0)
}

/// Viewport density tier used to select a sizing token pack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutScaleTier {
    /// Compact layout tuned for narrow plugin windows.
    Compact,
    /// Baseline layout for common desktop window sizes.
    Standard,
    /// Spacious layout tuned for wide desktop windows.
    Wide,
}

impl LayoutScaleTier {
    /// Resolve a scale tier from a logical viewport width.
    pub(crate) fn from_viewport_width(viewport_width: f32) -> Self {
        if viewport_width < 980.0 {
            Self::Compact
        } else if viewport_width > 2100.0 {
            Self::Wide
        } else {
            Self::Standard
        }
    }
}

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

/// Compact sizing tokens used by the native shell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SizingTokens {
    /// Minimum logical viewport width used by shell layout clamping.
    pub min_viewport_width: f32,
    /// Minimum logical viewport height used by shell layout clamping.
    pub min_viewport_height: f32,
    /// Root frame inset around the full shell viewport.
    pub frame_inset: f32,
    /// Shared gap between major shell panels.
    pub panel_gap: f32,
    /// Fixed top-bar band height.
    pub top_bar_height: f32,
    /// Height of the unified top-bar row inside the top bar.
    pub top_bar_title_row_height: f32,
    /// Minimum unified top-bar row height when vertical space is constrained.
    pub top_bar_title_row_min_height: f32,
    /// Reserved gap between title and controls rows. Zero keeps the top bar unified.
    pub top_bar_title_row_bottom_gap: f32,
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
    /// Maximum rendered browser rows per triage column.
    pub browser_rows_max_per_column: usize,
    /// Minimum width allowed for each triage column.
    pub column_min_width: f32,
    /// Height of the browser tab band.
    pub browser_tabs_height: f32,
    /// Minimum tabs-band height when browser panel space is constrained.
    pub browser_tabs_min_height: f32,
    /// Height of the browser toolbar band.
    pub browser_toolbar_height: f32,
    /// Minimum toolbar height when browser panel space is constrained.
    pub browser_toolbar_min_height: f32,
    /// Height of the browser table-header band.
    pub browser_table_header_height: f32,
    /// Minimum table-header height when browser panel space is constrained.
    pub browser_table_header_min_height: f32,
    /// Height of the browser footer band.
    pub browser_footer_height: f32,
    /// Minimum browser footer height to keep summary labels visible.
    pub browser_footer_min_height: f32,
    /// Maximum browser footer height to preserve row density.
    pub browser_footer_max_height: f32,
    /// Minimum width reserved for browser search controls.
    pub browser_search_field_min_width: f32,
    /// Preferred browser search width as a ratio of toolbar width.
    pub browser_search_field_ratio: f32,
    /// Width reserved for the browser row index column.
    pub browser_index_col_width: f32,
    /// Width reserved for the browser bucket/tag column.
    pub browser_bucket_col_width: f32,
    /// Shared panel inset for nested regions.
    pub panel_inset: f32,
    /// Small horizontal gutter applied to header/status text anchors.
    pub header_label_gutter: f32,
    /// Gap between browser rows.
    pub browser_row_gap: f32,
    /// Browser row card height.
    pub browser_row_height: f32,
    /// Gap between source rows.
    pub source_row_gap: f32,
    /// Source row card height.
    pub source_row_height: f32,
    /// Maximum number of source rows rendered in compact sidebar mode.
    pub source_rows_max: usize,
    /// Minimum number of source rows preserved when folder rows are also visible.
    pub source_rows_min_when_split: usize,
    /// Gap between folder rows.
    pub folder_row_gap: f32,
    /// Folder row card height.
    pub folder_row_height: f32,
    /// Maximum number of folder rows rendered in the sidebar tree.
    pub folder_rows_max: usize,
    /// Minimum number of folder rows preserved when source rows are visible.
    pub folder_rows_min: usize,
    /// Gap between source/folder sections in the sidebar.
    pub sidebar_section_gap: f32,
    /// Stroke width for source/folder section dividers.
    pub source_section_divider_width: f32,
    /// Shared vertical spacing between section headers and their row stacks.
    pub header_to_rows_gap: f32,
    /// Top padding inside row-stack regions.
    pub panel_section_padding_top: f32,
    /// Bottom padding inside row-stack regions.
    pub panel_section_padding_bottom: f32,
    /// Top block height reserved for folder section header + metadata.
    pub folder_header_block_height: f32,
    /// Recovery badge height in the folder section header.
    pub recovery_badge_height: f32,
    /// Minimum width reserved for folder recovery badges.
    pub recovery_badge_min_width: f32,
    /// Horizontal padding inside the recovery badge.
    pub recovery_badge_padding_x: f32,
    /// Horizontal indent step for nested folder rows.
    pub folder_indent_step: f32,
    /// Space between compact metadata text rows.
    pub text_row_gap: f32,
    /// Gap between title and metadata lines in stacked header labels.
    pub title_meta_gap: f32,
    /// Horizontal text inset inside row cards.
    pub text_inset_x: f32,
    /// Vertical text inset inside row cards.
    pub text_inset_y: f32,
    /// Extra horizontal inset used for row labels inside bordered cards.
    pub row_corner_inset: f32,
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
    /// Minimum content width reserved while clamping sidebar width.
    pub content_tail_min_width: f32,
    /// Minimum content height reserved for the browser panel.
    pub content_browser_min_height: f32,
    /// Minimum waveform card height when content is constrained.
    pub waveform_card_floor_height: f32,
    /// Browser action button width.
    pub action_button_width: f32,
    /// Browser action button height.
    pub action_button_height: f32,
    /// Gap between browser action buttons.
    pub action_button_gap: f32,
    /// Gap between top-bar title and action clusters.
    pub top_bar_cluster_gap: f32,
    /// Width of the top-bar volume meter track.
    pub top_volume_meter_width: f32,
    /// Height of the top-bar volume meter track.
    pub top_volume_meter_height: f32,
    /// Minimum width reserved for top-bar actions cluster.
    pub top_bar_action_cluster_min_width: f32,
    /// Maximum width reserved for top-bar actions cluster.
    pub top_bar_action_cluster_max_width: f32,
    /// Minimum width reserved for the title cluster beside actions.
    pub top_bar_action_cluster_title_reserve_width: f32,
    /// Horizontal gap between status-bar text segments.
    pub status_segment_gap: f32,
    /// Outer padding for modal overlays.
    pub overlay_padding: f32,
    /// Prompt dialog width.
    pub prompt_width: f32,
    /// Prompt dialog minimum height.
    pub prompt_min_height: f32,
    /// Overlay button width.
    pub overlay_button_width: f32,
    /// Overlay button height.
    pub overlay_button_height: f32,
    /// Progress bar height.
    pub progress_bar_height: f32,
    /// Drag overlay banner height.
    pub drag_overlay_height: f32,
    /// Sidebar action button width.
    pub sidebar_action_button_width: f32,
    /// Sidebar action button height.
    pub sidebar_action_button_height: f32,
    /// Gap between sidebar action buttons.
    pub sidebar_action_button_gap: f32,
    /// Border stroke width.
    pub border_width: f32,
    /// Stroke width used for focused controls.
    pub focus_stroke_width: f32,
    /// Hover fill blend factor for panel/card surfaces.
    pub hover_fill_alpha: f32,
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
    /// Build style tokens tuned for a viewport width and DPI scale factor.
    ///
    /// The input scale factor is clamped to a safe range so accidental outlier
    /// values cannot collapse or overinflate layout geometry. A small baseline
    /// multiplier is then applied to keep the default UI density slightly larger.
    pub(crate) fn for_viewport_with_scale(viewport_width: f32, ui_scale: f32) -> Self {
        let mut tokens = Self::for_tier(LayoutScaleTier::from_viewport_width(viewport_width));
        let effective_scale = clamp_ui_scale(clamp_ui_scale(ui_scale) * DEFAULT_UI_SCALE);
        tokens.sizing = tokens.sizing.with_ui_scale(effective_scale);
        tokens
    }

    /// Build style tokens tuned for a viewport width tier.
    pub(crate) fn for_viewport_width(viewport_width: f32) -> Self {
        Self::for_viewport_with_scale(viewport_width, 1.0)
    }

    /// Build style tokens for an explicit scale tier.
    pub(crate) fn for_tier(layout_tier: LayoutScaleTier) -> Self {
        let mut tokens = Self {
            layout_tier,
            clear_color: rgba(38, 38, 37, 255),
            bg_primary: rgba(19, 19, 19, 255),
            bg_secondary: rgba(26, 26, 25, 255),
            bg_tertiary: rgba(37, 37, 36, 255),
            surface_base: rgba(19, 19, 19, 255),
            surface_raised: rgba(31, 30, 29, 255),
            surface_overlay: rgba(43, 42, 40, 255),
            border: rgba(75, 74, 72, 255),
            border_emphasis: rgba(119, 118, 116, 255),
            source_section_divider: rgba(96, 94, 91, 255),
            source_recovery_badge_idle: rgba(63, 62, 60, 255),
            source_recovery_badge_active: rgba(223, 195, 121, 255),
            control_disabled_fill: rgba(50, 50, 48, 255),
            grid_strong: rgba(90, 89, 86, 255),
            grid_soft: rgba(60, 59, 57, 255),
            accent_mint: rgba(193, 193, 169, 255),
            accent_copper: rgba(227, 195, 122, 255),
            accent_trash: rgba(214, 110, 106, 255),
            accent_warning: rgba(239, 211, 149, 255),
            highlight_orange: rgba(223, 176, 92, 255),
            highlight_orange_soft: rgba(236, 204, 142, 255),
            highlight_blue: rgba(164, 164, 164, 255),
            highlight_blue_soft: rgba(184, 184, 184, 255),
            highlight_cyan: rgba(193, 193, 169, 255),
            highlight_cyan_soft: rgba(227, 226, 195, 255),
            text_primary: rgba(238, 234, 224, 255),
            text_muted: rgba(185, 180, 166, 255),
            state_hover_soft: 0.12,
            state_hover_strong: 0.20,
            state_selected_blend: 0.12,
            state_focus_pulse_blend: 0.24,
            motion_speed_transport: 2.6,
            motion_speed_idle: 1.2,
            motion_focus_wave_amp: 0.08,
            motion_focus_text_wave_amp: 0.04,
            scrim_soft_alpha: 172,
            scrim_modal_alpha: 188,
            sizing: SizingTokens {
                min_viewport_width: 620.0,
                min_viewport_height: 400.0,
                frame_inset: 0.0,
                panel_gap: 1.0,
                top_bar_height: 18.0,
                top_bar_title_row_height: 18.0,
                top_bar_title_row_min_height: 18.0,
                top_bar_title_row_bottom_gap: 0.0,
                status_bar_height: 16.0,
                sidebar_ratio: 0.16,
                sidebar_min_width: 158.0,
                sidebar_max_width: 214.0,
                content_min_width: 248.0,
                waveform_ratio: 0.34,
                waveform_min_height: 124.0,
                waveform_max_height: 248.0,
                column_gap: 5.0,
                browser_rows_max_per_column: 48,
                column_min_width: 40.0,
                browser_tabs_height: 17.0,
                browser_tabs_min_height: 16.0,
                browser_toolbar_height: 18.0,
                browser_toolbar_min_height: 18.0,
                browser_table_header_height: 17.0,
                browser_table_header_min_height: 16.0,
                browser_footer_height: 14.5,
                browser_footer_min_height: 14.0,
                browser_footer_max_height: 28.0,
                browser_search_field_min_width: 124.0,
                browser_search_field_ratio: 0.46,
                browser_index_col_width: 32.0,
                browser_bucket_col_width: 78.0,
                panel_inset: 4.0,
                header_label_gutter: 3.0,
                browser_row_gap: 0.0,
                browser_row_height: 16.5,
                source_row_gap: 1.0,
                source_row_height: 16.0,
                source_rows_max: 12,
                source_rows_min_when_split: 3,
                folder_row_gap: 1.0,
                folder_row_height: 15.4,
                folder_rows_max: 20,
                folder_rows_min: 4,
                sidebar_section_gap: 5.5,
                source_section_divider_width: 1.0,
                header_to_rows_gap: 3.0,
                panel_section_padding_top: 1.0,
                panel_section_padding_bottom: 1.0,
                folder_header_block_height: 29.0,
                recovery_badge_height: 14.0,
                recovery_badge_min_width: 56.0,
                recovery_badge_padding_x: 6.0,
                folder_indent_step: 11.0,
                text_row_gap: 1.5,
                title_meta_gap: 1.5,
                text_inset_x: 5.0,
                text_inset_y: 2.5,
                row_corner_inset: 1.2,
                source_header_block_height: 30.0,
                column_header_block_height: 18.0,
                waveform_header_block_height: 27.0,
                source_bottom_padding: 6.0,
                column_bottom_padding: 4.0,
                content_tail_min_width: 64.0,
                content_browser_min_height: 64.0,
                waveform_card_floor_height: 70.0,
                action_button_width: 48.0,
                action_button_height: 15.0,
                action_button_gap: 3.0,
                top_bar_cluster_gap: 8.0,
                top_volume_meter_width: 78.0,
                top_volume_meter_height: 6.0,
                top_bar_action_cluster_min_width: 250.0,
                top_bar_action_cluster_max_width: 480.0,
                top_bar_action_cluster_title_reserve_width: 72.0,
                status_segment_gap: 8.0,
                overlay_padding: 14.0,
                prompt_width: 420.0,
                prompt_min_height: 128.0,
                overlay_button_width: 84.0,
                overlay_button_height: 24.0,
                progress_bar_height: 12.0,
                drag_overlay_height: 24.0,
                sidebar_action_button_width: 50.0,
                sidebar_action_button_height: 15.0,
                sidebar_action_button_gap: 3.0,
                border_width: 1.0,
                focus_stroke_width: 1.2,
                hover_fill_alpha: 0.16,
                waveform_scan_step: 11.0,
                font_title: 12.5,
                font_header: 10.2,
                font_body: 9.0,
                font_meta: 8.6,
                font_status: 9.3,
                lamp_radius_base: 3.8,
                lamp_radius_amp: 1.8,
            },
        };

        if layout_tier == LayoutScaleTier::Compact {
            tokens.sizing.frame_inset = 0.0;
            tokens.sizing.panel_gap = 1.0;
            tokens.sizing.top_bar_height = 20.0;
            tokens.sizing.top_bar_title_row_height = 20.0;
            tokens.sizing.top_bar_title_row_min_height = 20.0;
            tokens.sizing.top_bar_title_row_bottom_gap = 0.0;
            tokens.sizing.status_bar_height = 19.0;
            tokens.sizing.sidebar_ratio = 0.23;
            tokens.sizing.sidebar_min_width = 168.0;
            tokens.sizing.sidebar_max_width = 252.0;
            tokens.sizing.content_min_width = 180.0;
            tokens.sizing.waveform_ratio = 0.36;
            tokens.sizing.waveform_min_height = 120.0;
            tokens.sizing.waveform_max_height = 220.0;
            tokens.sizing.column_gap = 5.0;
            tokens.sizing.browser_rows_max_per_column = 36;
            tokens.sizing.panel_inset = 5.0;
            tokens.sizing.header_label_gutter = 3.0;
            tokens.sizing.browser_tabs_height = 18.0;
            tokens.sizing.browser_toolbar_height = 19.0;
            tokens.sizing.browser_table_header_height = 18.0;
            tokens.sizing.browser_footer_height = 16.0;
            tokens.sizing.browser_search_field_min_width = 108.0;
            tokens.sizing.browser_search_field_ratio = 0.44;
            tokens.sizing.browser_index_col_width = 38.0;
            tokens.sizing.browser_bucket_col_width = 76.0;
            tokens.sizing.browser_row_gap = 0.0;
            tokens.sizing.browser_row_height = 15.8;
            tokens.sizing.source_row_gap = 2.0;
            tokens.sizing.source_row_height = 15.8;
            tokens.sizing.source_rows_max = 9;
            tokens.sizing.source_rows_min_when_split = 2;
            tokens.sizing.folder_row_gap = 2.0;
            tokens.sizing.folder_row_height = 14.8;
            tokens.sizing.folder_rows_max = 14;
            tokens.sizing.folder_rows_min = 3;
            tokens.sizing.sidebar_section_gap = 6.0;
            tokens.sizing.source_section_divider_width = 1.0;
            tokens.sizing.header_to_rows_gap = 3.0;
            tokens.sizing.panel_section_padding_top = 1.0;
            tokens.sizing.panel_section_padding_bottom = 1.0;
            tokens.sizing.folder_header_block_height = 28.0;
            tokens.sizing.recovery_badge_height = 13.0;
            tokens.sizing.recovery_badge_min_width = 48.0;
            tokens.sizing.recovery_badge_padding_x = 5.0;
            tokens.sizing.folder_indent_step = 10.0;
            tokens.sizing.title_meta_gap = 2.0;
            tokens.sizing.row_corner_inset = 1.0;
            tokens.sizing.source_header_block_height = 32.0;
            tokens.sizing.column_header_block_height = 19.0;
            tokens.sizing.waveform_header_block_height = 28.0;
            tokens.sizing.source_bottom_padding = 6.0;
            tokens.sizing.column_bottom_padding = 5.0;
            tokens.sizing.action_button_width = 48.0;
            tokens.sizing.action_button_height = 16.0;
            tokens.sizing.action_button_gap = 3.0;
            tokens.sizing.top_bar_cluster_gap = 8.0;
            tokens.sizing.top_volume_meter_width = 76.0;
            tokens.sizing.top_volume_meter_height = 7.0;
            tokens.sizing.top_bar_action_cluster_min_width = 240.0;
            tokens.sizing.top_bar_action_cluster_max_width = 420.0;
            tokens.sizing.status_segment_gap = 8.0;
            tokens.sizing.overlay_padding = 12.0;
            tokens.sizing.prompt_width = 360.0;
            tokens.sizing.prompt_min_height = 118.0;
            tokens.sizing.overlay_button_width = 78.0;
            tokens.sizing.overlay_button_height = 22.0;
            tokens.sizing.progress_bar_height = 10.0;
            tokens.sizing.drag_overlay_height = 22.0;
            tokens.sizing.sidebar_action_button_width = 50.0;
            tokens.sizing.sidebar_action_button_height = 16.0;
            tokens.sizing.sidebar_action_button_gap = 3.0;
            tokens.sizing.focus_stroke_width = 1.2;
            tokens.sizing.hover_fill_alpha = 0.14;
            tokens.state_hover_soft = 0.10;
            tokens.state_hover_strong = 0.16;
            tokens.state_selected_blend = 0.10;
            tokens.state_focus_pulse_blend = 0.20;
            tokens.motion_speed_transport = 2.2;
            tokens.motion_speed_idle = 1.0;
            tokens.motion_focus_wave_amp = 0.06;
            tokens.motion_focus_text_wave_amp = 0.03;
            tokens.scrim_soft_alpha = 164;
            tokens.scrim_modal_alpha = 180;
            tokens.sizing.waveform_scan_step = 11.0;
            tokens.sizing.font_title = 13.0;
            tokens.sizing.font_header = 11.0;
            tokens.sizing.font_body = 8.8;
            tokens.sizing.font_meta = 8.4;
            tokens.sizing.font_status = 9.2;
            tokens.surface_overlay = rgba(40, 39, 37, 255);
            tokens.border_emphasis = rgba(112, 111, 108, 255);
            tokens.source_section_divider = rgba(88, 86, 83, 255);
            tokens.source_recovery_badge_idle = rgba(57, 56, 53, 255);
            tokens.source_recovery_badge_active = tokens.accent_warning;
            tokens.control_disabled_fill = rgba(47, 46, 44, 255);
            return tokens;
        }

        if layout_tier == LayoutScaleTier::Wide {
            // Keep wide tier dense to preserve the classic Sempal table-first feel.
            tokens.sizing.frame_inset = 0.0;
            tokens.sizing.panel_gap = 1.0;
            tokens.sizing.top_bar_height = 19.0;
            tokens.sizing.top_bar_title_row_height = 19.0;
            tokens.sizing.top_bar_title_row_min_height = 19.0;
            tokens.sizing.top_bar_title_row_bottom_gap = 0.0;
            tokens.sizing.status_bar_height = 17.0;
            tokens.sizing.sidebar_ratio = 0.17;
            tokens.sizing.sidebar_min_width = 170.0;
            tokens.sizing.sidebar_max_width = 244.0;
            tokens.sizing.content_min_width = 250.0;
            tokens.sizing.waveform_ratio = 0.35;
            tokens.sizing.waveform_min_height = 130.0;
            tokens.sizing.waveform_max_height = 252.0;
            tokens.sizing.column_gap = 6.0;
            tokens.sizing.browser_rows_max_per_column = 64;
            tokens.sizing.panel_inset = 5.0;
            tokens.sizing.header_label_gutter = 3.5;
            tokens.sizing.browser_tabs_height = 18.0;
            tokens.sizing.browser_toolbar_height = 19.0;
            tokens.sizing.browser_table_header_height = 18.0;
            tokens.sizing.browser_footer_height = 16.0;
            tokens.sizing.browser_search_field_min_width = 140.0;
            tokens.sizing.browser_search_field_ratio = 0.47;
            tokens.sizing.browser_index_col_width = 36.0;
            tokens.sizing.browser_bucket_col_width = 84.0;
            tokens.sizing.browser_row_gap = 0.0;
            tokens.sizing.browser_row_height = 17.4;
            tokens.sizing.source_row_gap = 1.5;
            tokens.sizing.source_row_height = 16.9;
            tokens.sizing.source_rows_max = 13;
            tokens.sizing.source_rows_min_when_split = 4;
            tokens.sizing.folder_row_gap = 1.4;
            tokens.sizing.folder_row_height = 16.2;
            tokens.sizing.folder_rows_max = 22;
            tokens.sizing.folder_rows_min = 5;
            tokens.sizing.sidebar_section_gap = 6.0;
            tokens.sizing.source_section_divider_width = 1.0;
            tokens.sizing.header_to_rows_gap = 3.5;
            tokens.sizing.panel_section_padding_top = 1.5;
            tokens.sizing.panel_section_padding_bottom = 1.5;
            tokens.sizing.folder_header_block_height = 30.0;
            tokens.sizing.recovery_badge_height = 14.5;
            tokens.sizing.recovery_badge_min_width = 60.0;
            tokens.sizing.recovery_badge_padding_x = 7.0;
            tokens.sizing.folder_indent_step = 12.0;
            tokens.sizing.title_meta_gap = 2.0;
            tokens.sizing.row_corner_inset = 1.3;
            tokens.sizing.source_header_block_height = 33.0;
            tokens.sizing.column_header_block_height = 20.0;
            tokens.sizing.waveform_header_block_height = 29.0;
            tokens.sizing.source_bottom_padding = 7.0;
            tokens.sizing.column_bottom_padding = 5.0;
            tokens.sizing.action_button_width = 52.0;
            tokens.sizing.action_button_height = 16.5;
            tokens.sizing.action_button_gap = 3.0;
            tokens.sizing.top_bar_cluster_gap = 8.0;
            tokens.sizing.top_volume_meter_width = 82.0;
            tokens.sizing.top_volume_meter_height = 6.5;
            tokens.sizing.top_bar_action_cluster_min_width = 270.0;
            tokens.sizing.top_bar_action_cluster_max_width = 500.0;
            tokens.sizing.status_segment_gap = 9.0;
            tokens.sizing.overlay_padding = 14.0;
            tokens.sizing.prompt_width = 440.0;
            tokens.sizing.prompt_min_height = 130.0;
            tokens.sizing.overlay_button_width = 86.0;
            tokens.sizing.overlay_button_height = 24.0;
            tokens.sizing.progress_bar_height = 12.0;
            tokens.sizing.drag_overlay_height = 24.0;
            tokens.sizing.sidebar_action_button_width = 54.0;
            tokens.sizing.sidebar_action_button_height = 16.5;
            tokens.sizing.sidebar_action_button_gap = 3.0;
            tokens.sizing.focus_stroke_width = 1.3;
            tokens.sizing.hover_fill_alpha = 0.17;
            tokens.state_hover_soft = 0.12;
            tokens.state_hover_strong = 0.20;
            tokens.state_selected_blend = 0.13;
            tokens.state_focus_pulse_blend = 0.25;
            tokens.motion_speed_transport = 2.8;
            tokens.motion_speed_idle = 1.2;
            tokens.motion_focus_wave_amp = 0.08;
            tokens.motion_focus_text_wave_amp = 0.04;
            tokens.scrim_soft_alpha = 180;
            tokens.scrim_modal_alpha = 196;
            tokens.sizing.waveform_scan_step = 12.0;
            tokens.sizing.font_title = 13.0;
            tokens.sizing.font_header = 10.8;
            tokens.sizing.font_body = 9.4;
            tokens.sizing.font_meta = 8.9;
            tokens.sizing.font_status = 9.5;
            tokens.surface_overlay = rgba(47, 46, 44, 255);
            tokens.border_emphasis = rgba(128, 126, 123, 255);
            tokens.source_section_divider = rgba(102, 100, 96, 255);
            tokens.source_recovery_badge_idle = rgba(74, 72, 69, 255);
            tokens.source_recovery_badge_active = tokens.accent_warning;
            tokens.control_disabled_fill = rgba(53, 52, 49, 255);
        }
        tokens
    }
}

impl SizingTokens {
    /// Scale geometry and font tokens for a logical UI scale factor.
    ///
    /// This keeps tier selection stable while preserving density ratios and
    /// interaction values that should remain independent from geometry.
    fn with_ui_scale(mut self, ui_scale: f32) -> Self {
        let scale = clamp_ui_scale(ui_scale);
        if (scale - 1.0).abs() < f32::EPSILON {
            return self;
        }

        self.frame_inset *= scale;
        self.panel_gap *= scale;
        self.top_bar_height *= scale;
        self.top_bar_title_row_height *= scale;
        self.top_bar_title_row_min_height *= scale;
        self.top_bar_title_row_bottom_gap *= scale;
        self.status_bar_height *= scale;
        self.sidebar_min_width *= scale;
        self.sidebar_max_width *= scale;
        self.content_min_width *= scale;
        self.waveform_min_height *= scale;
        self.waveform_max_height *= scale;
        self.column_gap *= scale;
        self.column_min_width *= scale;
        self.browser_tabs_height *= scale;
        self.browser_tabs_min_height *= scale;
        self.browser_toolbar_height *= scale;
        self.browser_toolbar_min_height *= scale;
        self.browser_table_header_height *= scale;
        self.browser_table_header_min_height *= scale;
        self.browser_footer_height *= scale;
        self.browser_footer_min_height *= scale;
        self.browser_footer_max_height *= scale;
        self.browser_search_field_min_width *= scale;
        self.browser_index_col_width *= scale;
        self.browser_bucket_col_width *= scale;
        self.panel_inset *= scale;
        self.header_label_gutter *= scale;
        self.browser_row_gap *= scale;
        self.browser_row_height *= scale;
        self.source_row_gap *= scale;
        self.source_row_height *= scale;
        self.folder_row_gap *= scale;
        self.folder_row_height *= scale;
        self.sidebar_section_gap *= scale;
        self.source_section_divider_width *= scale;
        self.header_to_rows_gap *= scale;
        self.panel_section_padding_top *= scale;
        self.panel_section_padding_bottom *= scale;
        self.folder_header_block_height *= scale;
        self.recovery_badge_height *= scale;
        self.recovery_badge_min_width *= scale;
        self.recovery_badge_padding_x *= scale;
        self.folder_indent_step *= scale;
        self.text_row_gap *= scale;
        self.title_meta_gap *= scale;
        self.text_inset_x *= scale;
        self.text_inset_y *= scale;
        self.row_corner_inset *= scale;
        self.source_header_block_height *= scale;
        self.column_header_block_height *= scale;
        self.waveform_header_block_height *= scale;
        self.source_bottom_padding *= scale;
        self.column_bottom_padding *= scale;
        self.content_tail_min_width *= scale;
        self.content_browser_min_height *= scale;
        self.waveform_card_floor_height *= scale;
        self.action_button_width *= scale;
        self.action_button_height *= scale;
        self.action_button_gap *= scale;
        self.top_bar_cluster_gap *= scale;
        self.top_volume_meter_width *= scale;
        self.top_volume_meter_height *= scale;
        self.top_bar_action_cluster_min_width *= scale;
        self.top_bar_action_cluster_max_width *= scale;
        self.top_bar_action_cluster_title_reserve_width *= scale;
        self.status_segment_gap *= scale;
        self.overlay_padding *= scale;
        self.prompt_width *= scale;
        self.prompt_min_height *= scale;
        self.overlay_button_width *= scale;
        self.overlay_button_height *= scale;
        self.progress_bar_height *= scale;
        self.drag_overlay_height *= scale;
        self.sidebar_action_button_width *= scale;
        self.sidebar_action_button_height *= scale;
        self.sidebar_action_button_gap *= scale;
        self.border_width *= scale;
        self.focus_stroke_width *= scale;
        self.waveform_scan_step *= scale;
        self.font_title *= scale;
        self.font_header *= scale;
        self.font_body *= scale;
        self.font_meta *= scale;
        self.font_status *= scale;
        self.lamp_radius_base *= scale;
        self.lamp_radius_amp *= scale;
        self.frame_inset = 0.0;
        self.panel_gap = 1.0;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_UI_SCALE, LayoutScaleTier, StyleTokens};

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
