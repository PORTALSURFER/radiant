//! Geometry and typography tokens for the native shell.

use super::tier::{LayoutScaleTier, clamp_ui_scale};

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

/// Resolve the shell sizing pack for the requested viewport tier.
pub(super) fn sizing_for_tier(layout_tier: LayoutScaleTier) -> SizingTokens {
    let mut sizing = base_sizing();
    match layout_tier {
        LayoutScaleTier::Compact => apply_compact_sizing(&mut sizing),
        LayoutScaleTier::Wide => apply_wide_sizing(&mut sizing),
        LayoutScaleTier::Standard => {}
    }
    sizing
}

fn base_sizing() -> SizingTokens {
    SizingTokens {
        min_viewport_width: 620.0,
        min_viewport_height: 400.0,
        frame_inset: 0.0,
        panel_gap: 0.0,
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
        browser_search_field_min_width: 96.0,
        browser_search_field_ratio: 0.23,
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
        folder_row_gap: 0.0,
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
    }
}

fn apply_compact_sizing(sizing: &mut SizingTokens) {
    sizing.frame_inset = 0.0;
    sizing.panel_gap = 0.0;
    sizing.top_bar_height = 20.0;
    sizing.top_bar_title_row_height = 20.0;
    sizing.top_bar_title_row_min_height = 20.0;
    sizing.top_bar_title_row_bottom_gap = 0.0;
    sizing.status_bar_height = 19.0;
    sizing.sidebar_ratio = 0.23;
    sizing.sidebar_min_width = 168.0;
    sizing.sidebar_max_width = 252.0;
    sizing.content_min_width = 180.0;
    sizing.waveform_ratio = 0.36;
    sizing.waveform_min_height = 120.0;
    sizing.waveform_max_height = 220.0;
    sizing.column_gap = 5.0;
    sizing.browser_rows_max_per_column = 36;
    sizing.panel_inset = 5.0;
    sizing.header_label_gutter = 3.0;
    sizing.browser_tabs_height = 18.0;
    sizing.browser_toolbar_height = 19.0;
    sizing.browser_table_header_height = 18.0;
    sizing.browser_footer_height = 16.0;
    sizing.browser_search_field_min_width = 88.0;
    sizing.browser_search_field_ratio = 0.22;
    sizing.browser_index_col_width = 38.0;
    sizing.browser_bucket_col_width = 76.0;
    sizing.browser_row_gap = 0.0;
    sizing.browser_row_height = 15.8;
    sizing.source_row_gap = 2.0;
    sizing.source_row_height = 15.8;
    sizing.source_rows_max = 9;
    sizing.source_rows_min_when_split = 2;
    sizing.folder_row_gap = 0.0;
    sizing.folder_row_height = 14.8;
    sizing.folder_rows_max = 14;
    sizing.folder_rows_min = 3;
    sizing.sidebar_section_gap = 6.0;
    sizing.source_section_divider_width = 1.0;
    sizing.header_to_rows_gap = 3.0;
    sizing.panel_section_padding_top = 1.0;
    sizing.panel_section_padding_bottom = 1.0;
    sizing.folder_header_block_height = 28.0;
    sizing.recovery_badge_height = 13.0;
    sizing.recovery_badge_min_width = 48.0;
    sizing.recovery_badge_padding_x = 5.0;
    sizing.folder_indent_step = 10.0;
    sizing.title_meta_gap = 2.0;
    sizing.row_corner_inset = 1.0;
    sizing.source_header_block_height = 32.0;
    sizing.column_header_block_height = 19.0;
    sizing.waveform_header_block_height = 28.0;
    sizing.source_bottom_padding = 6.0;
    sizing.column_bottom_padding = 5.0;
    sizing.action_button_width = 48.0;
    sizing.action_button_height = 16.0;
    sizing.action_button_gap = 3.0;
    sizing.top_bar_cluster_gap = 8.0;
    sizing.top_volume_meter_width = 76.0;
    sizing.top_volume_meter_height = 7.0;
    sizing.top_bar_action_cluster_min_width = 240.0;
    sizing.top_bar_action_cluster_max_width = 420.0;
    sizing.status_segment_gap = 8.0;
    sizing.overlay_padding = 12.0;
    sizing.prompt_width = 360.0;
    sizing.prompt_min_height = 118.0;
    sizing.overlay_button_width = 78.0;
    sizing.overlay_button_height = 22.0;
    sizing.progress_bar_height = 10.0;
    sizing.drag_overlay_height = 22.0;
    sizing.sidebar_action_button_width = 50.0;
    sizing.sidebar_action_button_height = 16.0;
    sizing.sidebar_action_button_gap = 3.0;
    sizing.focus_stroke_width = 1.2;
    sizing.hover_fill_alpha = 0.14;
    sizing.waveform_scan_step = 11.0;
    sizing.font_title = 13.0;
    sizing.font_header = 11.0;
    sizing.font_body = 8.8;
    sizing.font_meta = 8.4;
    sizing.font_status = 9.2;
}

fn apply_wide_sizing(sizing: &mut SizingTokens) {
    sizing.frame_inset = 0.0;
    sizing.panel_gap = 0.0;
    sizing.top_bar_height = 19.0;
    sizing.top_bar_title_row_height = 19.0;
    sizing.top_bar_title_row_min_height = 19.0;
    sizing.top_bar_title_row_bottom_gap = 0.0;
    sizing.status_bar_height = 17.0;
    sizing.sidebar_ratio = 0.17;
    sizing.sidebar_min_width = 170.0;
    sizing.sidebar_max_width = 244.0;
    sizing.content_min_width = 250.0;
    sizing.waveform_ratio = 0.35;
    sizing.waveform_min_height = 130.0;
    sizing.waveform_max_height = 252.0;
    sizing.column_gap = 6.0;
    sizing.browser_rows_max_per_column = 64;
    sizing.panel_inset = 5.0;
    sizing.header_label_gutter = 3.5;
    sizing.browser_tabs_height = 18.0;
    sizing.browser_toolbar_height = 19.0;
    sizing.browser_table_header_height = 18.0;
    sizing.browser_footer_height = 16.0;
    sizing.browser_search_field_min_width = 108.0;
    sizing.browser_search_field_ratio = 0.24;
    sizing.browser_index_col_width = 36.0;
    sizing.browser_bucket_col_width = 84.0;
    sizing.browser_row_gap = 0.0;
    sizing.browser_row_height = 17.4;
    sizing.source_row_gap = 1.5;
    sizing.source_row_height = 16.9;
    sizing.source_rows_max = 13;
    sizing.source_rows_min_when_split = 4;
    sizing.folder_row_gap = 0.0;
    sizing.folder_row_height = 16.2;
    sizing.folder_rows_max = 22;
    sizing.folder_rows_min = 5;
    sizing.sidebar_section_gap = 6.0;
    sizing.source_section_divider_width = 1.0;
    sizing.header_to_rows_gap = 3.5;
    sizing.panel_section_padding_top = 1.5;
    sizing.panel_section_padding_bottom = 1.5;
    sizing.folder_header_block_height = 30.0;
    sizing.recovery_badge_height = 14.5;
    sizing.recovery_badge_min_width = 60.0;
    sizing.recovery_badge_padding_x = 7.0;
    sizing.folder_indent_step = 12.0;
    sizing.title_meta_gap = 2.0;
    sizing.row_corner_inset = 1.3;
    sizing.source_header_block_height = 33.0;
    sizing.column_header_block_height = 20.0;
    sizing.waveform_header_block_height = 29.0;
    sizing.source_bottom_padding = 7.0;
    sizing.column_bottom_padding = 5.0;
    sizing.action_button_width = 52.0;
    sizing.action_button_height = 16.5;
    sizing.action_button_gap = 3.0;
    sizing.top_bar_cluster_gap = 8.0;
    sizing.top_volume_meter_width = 82.0;
    sizing.top_volume_meter_height = 6.5;
    sizing.top_bar_action_cluster_min_width = 270.0;
    sizing.top_bar_action_cluster_max_width = 500.0;
    sizing.status_segment_gap = 9.0;
    sizing.overlay_padding = 14.0;
    sizing.prompt_width = 440.0;
    sizing.prompt_min_height = 130.0;
    sizing.overlay_button_width = 86.0;
    sizing.overlay_button_height = 24.0;
    sizing.progress_bar_height = 12.0;
    sizing.drag_overlay_height = 24.0;
    sizing.sidebar_action_button_width = 54.0;
    sizing.sidebar_action_button_height = 16.5;
    sizing.sidebar_action_button_gap = 3.0;
    sizing.focus_stroke_width = 1.3;
    sizing.hover_fill_alpha = 0.17;
    sizing.waveform_scan_step = 12.0;
    sizing.font_title = 13.0;
    sizing.font_header = 10.8;
    sizing.font_body = 9.4;
    sizing.font_meta = 8.9;
    sizing.font_status = 9.5;
}

impl SizingTokens {
    /// Scale geometry and font tokens for a logical UI scale factor.
    ///
    /// This keeps tier selection stable while preserving density ratios and
    /// interaction values that should remain independent from geometry.
    pub(crate) fn with_ui_scale(mut self, ui_scale: f32) -> Self {
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
        self.panel_gap = 0.0;
        self
    }
}
