//! Mutable interaction state and paint generation for the native shell.

use super::{
    layout::{ShellLayout, ShellNodeKind},
    layout_adapter::{
        BrowserTabsRects, SidebarRowCounts, compute_action_button_text_rect,
        compute_browser_footer_text_rect, compute_browser_header_text_layout,
        compute_browser_map_canvas_rect, compute_browser_map_header_text_layout,
        compute_browser_map_point_center, compute_browser_row_text_layout,
        compute_browser_tabs_rects, compute_browser_tabs_text_layout,
        compute_browser_toolbar_sections, compute_browser_toolbar_text_layout,
        compute_drag_overlay_text_layout, compute_drag_overlay_visual_layout,
        compute_progress_overlay_text_layout, compute_progress_overlay_visual_layout,
        compute_prompt_overlay_text_layout, compute_prompt_overlay_visual_layout,
        compute_row_index_at_point, compute_sidebar_action_button_rects,
        compute_sidebar_folder_header_layout, compute_sidebar_folder_row_text_rect,
        compute_sidebar_footer_text_layout, compute_sidebar_header_text_layout,
        compute_sidebar_recovery_badge_text_rect, compute_sidebar_row_sections,
        compute_sidebar_source_row_text_rect, compute_source_section_divider_rect,
        compute_status_text_line_rect, compute_top_bar_controls_sections,
        compute_top_bar_controls_text_layout, compute_update_action_button_rects,
        compute_waveform_annotation_rects, compute_waveform_header_text_layout,
    },
    paint::{DrawImage, FillCircle, FillRect, NativeViewFrame, Primitive, TextAlign, TextRun},
    style::{SizingTokens, StyleTokens},
};
use crate::app::{AppModel, BrowserRowModel, DirtySegments, NativeMotionModel, UiAction};
use crate::gui::{
    input::KeyCode,
    types::{ImageRgba, Point, Rect, Rgba8},
};
use std::{
    cell::RefCell,
    collections::HashMap,
    hash::{Hash, Hasher},
};

mod browser_rows;
mod cache;
mod cache_types;
mod frame_build;
mod hit_testing;
mod model_sync;
mod options_panel;
mod overlays;
mod svg_icons;
mod text_fields;
mod toolbar_helpers;
mod waveform_segments;

use self::{
    browser_rows::*, cache_types::*, hit_testing::*, options_panel::*, overlays::*, svg_icons::*,
    text_fields::*, toolbar_helpers::*, waveform_segments::*,
};
pub(crate) use self::{
    cache_types::{
        ChromeMotionOverlayFingerprint, CursorMoveEffect, StateOverlayFingerprint,
        StaticFrameSegment, StaticFrameSegments, WaveformMotionOverlayFingerprint,
    },
    text_fields::TextFieldVisualState,
};

/// Maximum retained entries for browser-row text truncation outputs.
const BROWSER_ROW_TRUNCATION_CACHE_CAPACITY: usize = 1024;
/// Text glyph shown before browser sample labels whose backing files are missing.
const BROWSER_MISSING_SAMPLE_MARKER: &str = "!";
/// Red marker color used to flag missing browser sample files.
const BROWSER_MISSING_SAMPLE_MARKER_COLOR: Rgba8 = Rgba8 {
    r: 236,
    g: 84,
    b: 84,
    a: 255,
};
/// Maximum retained ghost lines for the dynamic waveform playhead trail.
const PLAYHEAD_TRAIL_MAX_SAMPLES: usize = 768;
/// Number of seconds used to fade one retained playhead ghost line.
///
/// Time-based aging avoids visible fade quantization when redraw cadence varies.
const PLAYHEAD_TRAIL_FADE_SECONDS: f32 = 1.2;
/// Maximum opacity used for the newest retained trail sample behind the live playhead.
///
/// The active playhead line itself remains fully opaque; only the ghost trail fades from
/// this half-strength head value down to zero.
const PLAYHEAD_TRAIL_HEAD_ALPHA: f32 = 0.2;
/// Maximum inserted in-between samples per motion frame for smooth trails.
const PLAYHEAD_TRAIL_MAX_INTERPOLATED_STEPS: usize = 192;
/// Largest contiguous frame delta treated as normal transport motion.
const PLAYHEAD_TRAIL_MAX_CONTIGUOUS_DELTA_MICROS: u64 = 120_000;
/// Minimum synthetic time delta used when motion redraws arrive in the same tick.
const PLAYHEAD_TRAIL_MIN_INTERPOLATED_DELTA_SECONDS: f32 = 1.0 / 240.0;
/// Number of animation ticks used for one waveform-toolbar click flash.
const WAVEFORM_TOOLBAR_FLASH_TICKS: u8 = 6;
/// Number of animation ticks used for the sidebar source-add button click flash.
const SOURCE_ADD_BUTTON_FLASH_TICKS: u8 = 6;
/// Rating-filter chip levels shown left-to-right in the browser toolbar.
const BROWSER_RATING_FILTER_LEVELS: [i8; 8] = [-3, -2, -1, 0, 1, 2, 3, 4];
/// Additional hit slop for the narrow browser scrollbar thumb.
const BROWSER_SCROLLBAR_THUMB_HIT_SLOP: f32 = 3.0;

/// Mutable interaction + animation state for the native shell.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NativeShellState {
    selected_column: usize,
    hovered: Option<ShellNodeKind>,
    hovered_browser_visible_row: Option<usize>,
    hovered_browser_rating_filter_level: Option<i8>,
    hovered_browser_search_field: bool,
    browser_search_editor_visual: Option<TextFieldVisualState>,
    hovered_folder_row_index: Option<usize>,
    hovered_source_add_button: bool,
    hovered_status_options_button: bool,
    hovered_waveform_toolbar_hint: Option<WaveformToolbarHoverHint>,
    waveform_toolbar_flash: Option<WaveformToolbarFlash>,
    source_add_button_flash_ticks: u8,
    status_options_button_flash_ticks: u8,
    hovered_waveform_resize_edge: Option<WaveformResizeHoverEdge>,
    waveform_bpm_input_active: bool,
    waveform_bpm_input_display: Option<String>,
    waveform_bpm_editor_visual: Option<TextFieldVisualState>,
    waveform_hover_x: Option<f32>,
    last_waveform_playhead_micros: Option<u32>,
    last_waveform_view_window: Option<(u32, u32)>,
    playhead_trail_samples: Vec<PlayheadTrailSample>,
    playhead_trail_elapsed_seconds: f32,
    transport_running: bool,
    has_focus_emphasis: bool,
    startup_frame_ticks: u8,
    pulse_phase: f32,
    source_context_menu: Option<SourceContextMenuState>,
    source_row_rects: Vec<Rect>,
    source_row_cache_key: Option<SidebarRowsCacheKey>,
    folder_row_rects: Vec<Rect>,
    folder_row_cache_key: Option<SidebarRowsCacheKey>,
    browser_rows: Vec<CachedBrowserRow>,
    browser_rows_window_start: usize,
    browser_rows_cache_key: Option<BrowserRowsCacheKey>,
    browser_action_buttons: Vec<ActionButton>,
    browser_column_chips: Vec<BrowserColumnChip>,
    browser_toolbar_layout: Option<BrowserToolbarLayout>,
    browser_action_hit_test_cache_key: Option<BrowserActionHitTestCacheKey>,
    waveform_toolbar_buttons: Vec<WaveformToolbarButton>,
    waveform_toolbar_hit_test_cache_key: Option<WaveformToolbarHitTestCacheKey>,
    browser_row_truncation_cache: BrowserRowTruncationCache,
    browser_row_truncation_cache_key: Option<BrowserRowTruncationCacheKey>,
    browser_row_truncation_frame_counts: BrowserRowTruncationFrameCounts,
}

impl NativeShellState {
    /// Create a default shell state.
    pub(crate) fn new() -> Self {
        Self {
            selected_column: 1,
            hovered: None,
            hovered_browser_visible_row: None,
            hovered_browser_rating_filter_level: None,
            hovered_browser_search_field: false,
            browser_search_editor_visual: None,
            hovered_folder_row_index: None,
            hovered_source_add_button: false,
            hovered_status_options_button: false,
            hovered_waveform_toolbar_hint: None,
            waveform_toolbar_flash: None,
            source_add_button_flash_ticks: 0,
            status_options_button_flash_ticks: 0,
            hovered_waveform_resize_edge: None,
            waveform_bpm_input_active: false,
            waveform_bpm_input_display: None,
            waveform_bpm_editor_visual: None,
            waveform_hover_x: None,
            last_waveform_playhead_micros: None,
            last_waveform_view_window: None,
            playhead_trail_samples: Vec::new(),
            playhead_trail_elapsed_seconds: 0.0,
            transport_running: true,
            has_focus_emphasis: false,
            startup_frame_ticks: 2,
            pulse_phase: 0.0,
            source_context_menu: None,
            source_row_rects: Vec::new(),
            source_row_cache_key: None,
            folder_row_rects: Vec::new(),
            folder_row_cache_key: None,
            browser_rows: Vec::new(),
            browser_rows_window_start: 0,
            browser_rows_cache_key: None,
            browser_action_buttons: Vec::new(),
            browser_column_chips: Vec::new(),
            browser_toolbar_layout: None,
            browser_action_hit_test_cache_key: None,
            waveform_toolbar_buttons: Vec::new(),
            waveform_toolbar_hit_test_cache_key: None,
            browser_row_truncation_cache: BrowserRowTruncationCache::default(),
            browser_row_truncation_cache_key: None,
            browser_row_truncation_frame_counts: BrowserRowTruncationFrameCounts::default(),
        }
    }

    /// Return whether the shell currently needs continuous animation.
    /// Focus emphasis is intentionally not included so selection and focus rendering
    /// remains static without forcing redraws when transport is idle.
    pub(crate) fn needs_animation(&self) -> bool {
        self.animation_reasons().needs_animation()
    }

    fn animation_reasons(&self) -> NativeAnimationReasons {
        NativeAnimationReasons {
            transport_running: self.transport_running,
            startup_frame_tick: self.startup_frame_ticks > 0,
            playhead_trail_active: !self.playhead_trail_samples.is_empty(),
            waveform_toolbar_flash_active: self.waveform_toolbar_flash.is_some(),
            source_add_button_flash_active: self.source_add_button_flash_ticks > 0,
            status_options_button_flash_active: self.status_options_button_flash_ticks > 0,
        }
    }

    /// Return whether playback transport is currently reported as running.
    pub(crate) fn is_transport_running(&self) -> bool {
        self.transport_running
    }

    /// Handle pointer movement and classify which overlay bucket changed.
    pub(crate) fn handle_cursor_move_effect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> CursorMoveEffect {
        let next_hover = layout.hit_test(point);
        let next_hovered_browser_row =
            self.resolve_hovered_browser_row(layout, model, point, next_hover);
        let next_hovered_browser_rating_filter_level =
            self.resolve_hovered_browser_rating_filter_level(layout, model, point);
        let next_hovered_browser_search_field =
            self.resolve_hovered_browser_search_field(layout, model, point);
        let next_hovered_folder_row =
            self.resolve_hovered_folder_row(layout, model, point, next_hover);
        let next_hovered_source_add_button =
            self.resolve_hovered_source_add_button(layout, point, next_hover);
        let next_hovered_status_options_button =
            self.resolve_hovered_status_options_button(layout, point, next_hover);
        let next_hovered_waveform_toolbar_hint =
            self.resolve_hovered_waveform_toolbar_hint(layout, model, point, next_hover);
        let next_hovered_waveform_resize_edge =
            hovered_waveform_resize_edge_for_point(layout, model, point, next_hover);
        let next_waveform_hover_x = waveform_hover_x_for_point(layout, next_hover, point);
        let hover_changed = next_hover != self.hovered;
        let browser_row_changed = next_hovered_browser_row != self.hovered_browser_visible_row;
        let browser_rating_filter_changed =
            next_hovered_browser_rating_filter_level != self.hovered_browser_rating_filter_level;
        let browser_search_field_changed =
            next_hovered_browser_search_field != self.hovered_browser_search_field;
        let folder_row_changed = next_hovered_folder_row != self.hovered_folder_row_index;
        let source_add_button_changed =
            next_hovered_source_add_button != self.hovered_source_add_button;
        let status_options_button_changed =
            next_hovered_status_options_button != self.hovered_status_options_button;
        let waveform_toolbar_hint_changed =
            next_hovered_waveform_toolbar_hint != self.hovered_waveform_toolbar_hint;
        let waveform_resize_edge_changed =
            next_hovered_waveform_resize_edge != self.hovered_waveform_resize_edge;
        let waveform_hover_changed =
            next_waveform_hover_x.map(f32::to_bits) != self.waveform_hover_x.map(f32::to_bits);
        if !hover_changed
            && !browser_row_changed
            && !browser_rating_filter_changed
            && !browser_search_field_changed
            && !folder_row_changed
            && !source_add_button_changed
            && !status_options_button_changed
            && !waveform_toolbar_hint_changed
            && !waveform_resize_edge_changed
            && !waveform_hover_changed
        {
            return CursorMoveEffect::None;
        }
        self.hovered = next_hover;
        self.hovered_browser_visible_row = next_hovered_browser_row;
        self.hovered_browser_rating_filter_level = next_hovered_browser_rating_filter_level;
        self.hovered_browser_search_field = next_hovered_browser_search_field;
        self.hovered_folder_row_index = next_hovered_folder_row;
        self.hovered_source_add_button = next_hovered_source_add_button;
        self.hovered_status_options_button = next_hovered_status_options_button;
        self.hovered_waveform_toolbar_hint = next_hovered_waveform_toolbar_hint;
        self.hovered_waveform_resize_edge = next_hovered_waveform_resize_edge;
        self.waveform_hover_x = next_waveform_hover_x;
        if waveform_hover_changed
            && !hover_changed
            && !browser_row_changed
            && !browser_rating_filter_changed
            && !browser_search_field_changed
            && !folder_row_changed
            && !source_add_button_changed
            && !status_options_button_changed
            && !waveform_toolbar_hint_changed
            && !waveform_resize_edge_changed
        {
            CursorMoveEffect::WaveformHoverOnly
        } else {
            CursorMoveEffect::GeneralOverlay
        }
    }

    fn resolve_hovered_browser_row(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
        hover: Option<ShellNodeKind>,
    ) -> Option<usize> {
        if model.map.active || hover != Some(ShellNodeKind::BrowserTable) {
            return None;
        }
        let style = style_for_layout(layout);
        let rows = self.cached_browser_rows(layout, &style, model);
        row_index_for_visible_rows(rows, point, layout.browser_rows)
            .map(|index| rows[index].visible_row)
    }

    fn resolve_hovered_folder_row(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
        hover: Option<ShellNodeKind>,
    ) -> Option<usize> {
        if hover != Some(ShellNodeKind::Sidebar) {
            return None;
        }
        let style = style_for_layout(layout);
        let rows = self.cached_folder_row_rects(layout, &style, model);
        compute_row_index_at_point(rows, point)
    }

    fn resolve_hovered_browser_search_field(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        let style = style_for_layout(layout);
        let (_, _, toolbar) = self.cached_browser_action_hit_test(layout, &style, model);
        toolbar.search_field.width() > 1.0 && toolbar.search_field.contains(point)
    }

    fn resolve_hovered_browser_rating_filter_level(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<i8> {
        let style = style_for_layout(layout);
        let (_, _, toolbar) = self.cached_browser_action_hit_test(layout, &style, model);
        browser_rating_filter_level_at_point(toolbar.rating_filter_chips, point)
    }

    fn resolve_hovered_source_add_button(
        &self,
        layout: &ShellLayout,
        point: Point,
        hover: Option<ShellNodeKind>,
    ) -> bool {
        if hover != Some(ShellNodeKind::Sidebar) {
            return false;
        }
        source_add_button_rect(layout.sidebar_header, style_for_layout(layout).sizing)
            .is_some_and(|rect| rect.contains(point))
    }

    fn resolve_hovered_status_options_button(
        &self,
        layout: &ShellLayout,
        point: Point,
        hover: Option<ShellNodeKind>,
    ) -> bool {
        if hover != Some(ShellNodeKind::TopBar) {
            return false;
        }
        status_options_button_rect(
            layout.top_bar_action_cluster,
            style_for_layout(layout).sizing,
        )
        .is_some_and(|rect| rect.contains(point))
    }

    fn resolve_hovered_waveform_toolbar_hint(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
        hover: Option<ShellNodeKind>,
    ) -> Option<WaveformToolbarHoverHint> {
        if hover != Some(ShellNodeKind::WaveformCard) {
            return None;
        }
        let style = style_for_layout(layout);
        let motion_model = NativeMotionModel::from_app_model(model);
        self.cached_waveform_toolbar_buttons(layout, &style, &motion_model)
            .iter()
            .find(|button| button.rect.contains(point))
            .and_then(|button| waveform_toolbar_hover_hint(button.label))
    }

    /// Handle a primary button click at the pointer position.
    pub(crate) fn handle_primary_click(&mut self, layout: &ShellLayout, point: Point) -> bool {
        let Some(column) = layout.column_at_point(point) else {
            return false;
        };
        if self.selected_column == column {
            return false;
        }
        self.selected_column = column;
        true
    }

    /// Handle backend-agnostic key input.
    pub(crate) fn handle_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::ArrowLeft => {
                self.selected_column = (self.selected_column + 2) % 3;
                true
            }
            KeyCode::ArrowRight => {
                self.selected_column = (self.selected_column + 1) % 3;
                true
            }
            KeyCode::Num1 => {
                if self.selected_column == 0 {
                    false
                } else {
                    self.selected_column = 0;
                    true
                }
            }
            KeyCode::Num2 => {
                if self.selected_column == 1 {
                    false
                } else {
                    self.selected_column = 1;
                    true
                }
            }
            KeyCode::Num3 => {
                if self.selected_column == 2 {
                    false
                } else {
                    self.selected_column = 2;
                    true
                }
            }
            _ => false,
        }
    }

    /// Resolve a rendered source-row index for a point within the sidebar.
    pub(crate) fn source_row_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<usize> {
        let style = style_for_layout(layout);
        let source_rows = self.cached_source_row_rects(layout, &style, model);
        compute_row_index_at_point(source_rows, point)
    }

    /// Resolve a rendered folder-row index for a point within the sidebar.
    pub(crate) fn folder_row_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<usize> {
        let style = style_for_layout(layout);
        let folder_rows = self.cached_folder_row_rects(layout, &style, model);
        compute_row_index_at_point(folder_rows, point)
    }

    /// Open the transient source context menu for one source row.
    pub(crate) fn open_source_context_menu_for_row(&mut self, row_index: usize, anchor: Point) {
        self.source_context_menu = Some(SourceContextMenuState { row_index, anchor });
    }

    /// Close the transient source context menu.
    ///
    /// Returns `true` when a visible menu was dismissed.
    pub(crate) fn close_source_context_menu(&mut self) -> bool {
        if self.source_context_menu.is_some() {
            self.source_context_menu = None;
            return true;
        }
        false
    }

    /// Resolve one source context-menu action at a pointer location.
    pub(crate) fn source_context_menu_action_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        let (_, buttons) =
            source_context_menu_spec(layout, &style, model, self.source_context_menu)?;
        buttons
            .into_iter()
            .find(|button| button.enabled && button.rect.contains(point))
            .map(|button| button.action.clone())
    }

    /// Return `true` when a point lands inside the visible source context menu panel.
    #[cfg(test)]
    pub(crate) fn source_context_menu_contains_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        let style = style_for_layout(layout);
        let Some((panel_rect, _)) =
            source_context_menu_spec(layout, &style, model, self.source_context_menu)
        else {
            return false;
        };
        panel_rect.contains(point)
    }

    /// Return rendered source-row rectangles for geometry tests.
    #[cfg(test)]
    pub(crate) fn rendered_source_row_rects(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Vec<Rect> {
        let style = style_for_layout(layout);
        self.cached_source_row_rects(layout, &style, model).to_vec()
    }

    /// Return rendered folder-row rectangles for geometry tests.
    #[cfg(test)]
    pub(crate) fn rendered_folder_row_rects(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Vec<Rect> {
        let style = style_for_layout(layout);
        self.cached_folder_row_rects(layout, &style, model).to_vec()
    }

    /// Return a source-action button rect for the provided action in tests.
    #[cfg(test)]
    pub(crate) fn source_action_button_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        action: UiAction,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        source_action_buttons(layout, &style, model)
            .into_iter()
            .find(|button| button.action == action)
            .map(|button| button.rect)
    }

    /// Return the sidebar-header add-source button rect in tests.
    #[cfg(test)]
    pub(crate) fn source_add_button_rect(&self, layout: &ShellLayout) -> Option<Rect> {
        source_add_button_rect(layout.sidebar_header, style_for_layout(layout).sizing)
    }

    /// Return the top-right options button rect in tests.
    #[cfg(test)]
    pub(crate) fn status_options_button_rect(&self, layout: &ShellLayout) -> Option<Rect> {
        status_options_button_rect(
            layout.top_bar_action_cluster,
            style_for_layout(layout).sizing,
        )
    }

    /// Return whether a point falls inside the visible options panel.
    #[cfg(test)]
    pub(crate) fn options_panel_contains_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        options_panel_contains_point(layout, &style_for_layout(layout), model, point)
    }

    /// Return whether a point falls inside the visible options panel.
    pub(crate) fn options_panel_contains_point_live(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        options_panel_contains_point(layout, &style_for_layout(layout), model, point)
    }

    /// Resolve a click inside the visible options panel.
    pub(crate) fn options_panel_action_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        options_panel_action_at_point(layout, &style_for_layout(layout), model, point)
    }

    /// Return a source-context-menu button rect for one action in tests.
    #[cfg(test)]
    pub(crate) fn source_context_menu_button_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        action: UiAction,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let (_, buttons) =
            source_context_menu_spec(layout, &style, model, self.source_context_menu)?;
        buttons
            .into_iter()
            .find(|button| button.action == action)
            .map(|button| button.rect)
    }

    /// Return a browser column-chip rect for one column index in tests.
    #[cfg(test)]
    pub(crate) fn browser_column_chip_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        column: usize,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let buttons = browser_action_buttons(layout, &style, model);
        browser_column_chips(layout, &style, model, &buttons)
            .into_iter()
            .find(|chip| chip.column == column)
            .map(|chip| chip.rect)
    }

    /// Return a waveform-toolbar button rect for one control label in tests.
    #[cfg(test)]
    pub(crate) fn waveform_toolbar_button_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        label: &'static str,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let motion_model = NativeMotionModel::from_app_model(model);
        waveform_toolbar_buttons(
            layout,
            &style,
            &motion_model,
            self.waveform_bpm_input_active,
            self.waveform_bpm_input_display.as_deref(),
        )
        .into_iter()
        .find(|button| button.label == label)
        .map(|button| button.rect)
    }

    /// Resolve a source-management action button click into a native UI action.
    pub(crate) fn source_action_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        if source_add_button_rect(layout.sidebar_header, style.sizing)
            .is_some_and(|rect| rect.contains(point))
        {
            self.trigger_source_add_button_flash();
            return Some(UiAction::OpenAddSourceDialog);
        }
        source_action_buttons(layout, &style, model)
            .into_iter()
            .find(|button| button.enabled && button.rect.contains(point))
            .map(|button| button.action)
    }

    /// Resolve a rendered browser visible-row index for a point in the triage pane.
    pub(crate) fn browser_row_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<usize> {
        if model.map.active {
            return None;
        }
        let style = style_for_layout(layout);
        let rows = self.cached_browser_rows(layout, &style, model);
        row_index_for_visible_rows(rows, point, layout.browser_rows)
            .map(|index| rows[index].visible_row)
    }

    /// Return the current rendered browser viewport length.
    pub(crate) fn browser_viewport_len(&mut self, layout: &ShellLayout, model: &AppModel) -> usize {
        let style = style_for_layout(layout);
        self.cached_browser_rows(layout, &style, model)
            .len()
            .min(model.browser.visible_count)
    }

    /// Return the pointer's offset within the browser scrollbar thumb when hovered.
    pub(crate) fn browser_scrollbar_thumb_offset_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<f32> {
        let (scrollbar, _) = self.cached_browser_scrollbar(layout, model)?;
        let hit_rect = Rect::from_min_max(
            Point::new(
                scrollbar.track.min.x - BROWSER_SCROLLBAR_THUMB_HIT_SLOP,
                scrollbar.thumb.min.y - BROWSER_SCROLLBAR_THUMB_HIT_SLOP,
            ),
            Point::new(
                scrollbar.track.max.x + BROWSER_SCROLLBAR_THUMB_HIT_SLOP,
                scrollbar.thumb.max.y + BROWSER_SCROLLBAR_THUMB_HIT_SLOP,
            ),
        );
        hit_rect
            .contains(point)
            .then_some((point.y - scrollbar.thumb.min.y).clamp(0.0, scrollbar.thumb.height()))
    }

    /// Resolve the browser viewport start row for an active scrollbar-thumb drag.
    pub(crate) fn browser_scrollbar_view_start_for_drag(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        pointer_y: f32,
        thumb_pointer_offset_y: f32,
    ) -> Option<usize> {
        let (scrollbar, viewport_len) = self.cached_browser_scrollbar(layout, model)?;
        browser_scrollbar_view_start_for_pointer(
            scrollbar,
            viewport_len,
            model.browser.visible_count,
            pointer_y,
            thumb_pointer_offset_y,
        )
    }

    /// Resolve the browser viewport start for a click inside the scrollbar track.
    ///
    /// Track clicks jump the thumb so its center aligns with the clicked
    /// location, matching the visual expectation that the handle should move to
    /// the requested position immediately.
    pub(crate) fn browser_scrollbar_view_start_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<usize> {
        let (scrollbar, viewport_len) = self.cached_browser_scrollbar(layout, model)?;
        if !scrollbar.track.contains(point) || scrollbar.thumb.contains(point) {
            return None;
        }
        browser_scrollbar_view_start_for_pointer(
            scrollbar,
            viewport_len,
            model.browser.visible_count,
            point.y,
            scrollbar.thumb.height() * 0.5,
        )
    }

    /// Return the pointer's offset within the waveform scrollbar thumb when hovered.
    pub(crate) fn waveform_scrollbar_thumb_offset_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<f32> {
        let scrollbar = waveform_scrollbar_layout(
            layout.waveform_plot,
            model.waveform.view_start_micros,
            model.waveform.view_end_micros,
        )?;
        scrollbar
            .thumb
            .contains(point)
            .then_some((point.x - scrollbar.thumb.min.x).clamp(0.0, scrollbar.thumb.width()))
    }

    /// Resolve the waveform viewport center for an active scrollbar-thumb drag.
    pub(crate) fn waveform_scrollbar_view_center_for_drag(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        pointer_x: f32,
        thumb_pointer_offset_x: f32,
    ) -> Option<u32> {
        let scrollbar = waveform_scrollbar_layout(
            layout.waveform_plot,
            model.waveform.view_start_micros,
            model.waveform.view_end_micros,
        )?;
        waveform_scrollbar_center_for_pointer(
            scrollbar,
            model.waveform.view_start_micros,
            model.waveform.view_end_micros,
            pointer_x,
            thumb_pointer_offset_x,
        )
    }

    /// Resolve the waveform viewport center for a click inside the scrollbar track.
    pub(crate) fn waveform_scrollbar_view_center_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<u32> {
        let scrollbar = waveform_scrollbar_layout(
            layout.waveform_plot,
            model.waveform.view_start_micros,
            model.waveform.view_end_micros,
        )?;
        if !scrollbar.track.contains(point) || scrollbar.thumb.contains(point) {
            return None;
        }
        waveform_scrollbar_center_for_pointer(
            scrollbar,
            model.waveform.view_start_micros,
            model.waveform.view_end_micros,
            point.x,
            scrollbar.thumb.width() * 0.5,
        )
    }

    /// Resolve a browser action-strip click into a native UI action.
    pub(crate) fn browser_action_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
        alt_down: bool,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        let (buttons, chips, toolbar) = self.cached_browser_action_hit_test(layout, &style, model);
        if let Some(level) =
            browser_rating_filter_level_at_point(toolbar.rating_filter_chips, point)
        {
            return Some(UiAction::ToggleBrowserRatingFilter {
                level,
                invert: alt_down,
            });
        }
        if toolbar.search_field.width() > 1.0 && toolbar.search_field.contains(point) {
            return Some(UiAction::FocusBrowserSearch);
        }
        if let Some(action) = chips
            .into_iter()
            .find(|chip| chip.rect.contains(point))
            .map(|chip| UiAction::SelectColumn { index: chip.column })
        {
            return Some(action);
        }
        buttons
            .into_iter()
            .find(|button| button.enabled && button.rect.contains(point))
            .map(|button| button.action.clone())
    }

    /// Resolve a browser tab click into a list/map tab selection action.
    pub(crate) fn browser_tab_action_at_point(
        &self,
        layout: &ShellLayout,
        point: Point,
    ) -> Option<UiAction> {
        let tabs: BrowserTabsRects =
            compute_browser_tabs_rects(layout.browser_tabs, style_for_layout(layout).sizing);
        if tabs.samples.contains(point) {
            return Some(UiAction::SetBrowserTab { map: false });
        }
        if tabs.map.contains(point) {
            return Some(UiAction::SetBrowserTab { map: true });
        }
        None
    }

    /// Resolve a waveform-toolbar control click into a native UI action.
    pub(crate) fn waveform_toolbar_action_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let motion_model = NativeMotionModel::from_app_model(model);
        self.waveform_toolbar_action_at_point_with_motion(layout, &motion_model, point)
    }

    /// Resolve a waveform-toolbar control click into a native UI action.
    pub(crate) fn waveform_toolbar_action_at_point_with_motion(
        &mut self,
        layout: &ShellLayout,
        motion_model: &NativeMotionModel,
        point: Point,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        let resolved = self
            .cached_waveform_toolbar_buttons(layout, &style, motion_model)
            .into_iter()
            .find(|button| button.enabled && button.rect.contains(point))
            .map(|button| {
                (
                    waveform_toolbar_hover_hint(button.label),
                    button.action.clone(),
                )
            });
        if let Some((Some(hint), _)) = resolved.as_ref() {
            self.trigger_waveform_toolbar_flash(*hint);
        }
        resolved.and_then(|(_, action)| action)
    }

    /// Resolve a click inside the status-bar options button to a native options action.
    pub(crate) fn status_options_action_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let Some(button_rect) = status_options_button_rect(
            layout.top_bar_action_cluster,
            style_for_layout(layout).sizing,
        ) else {
            return None;
        };
        if !button_rect.contains(point) {
            return None;
        }
        self.trigger_status_options_button_flash();
        Some(if model.options_panel.visible {
            UiAction::CloseOptionsPanel
        } else {
            UiAction::OpenOptionsMenu
        })
    }

    /// Resolve a click inside the top-bar volume meter to a volume action.
    pub(crate) fn top_bar_volume_action_at_point(
        &self,
        layout: &ShellLayout,
        point: Point,
    ) -> Option<UiAction> {
        let controls = top_bar_controls_layout(layout, style_for_layout(layout).sizing);
        if !controls.active || !controls.volume_meter.contains(point) {
            return None;
        }
        Some(volume_action_for_meter(controls.volume_meter, point))
    }

    /// Resolve a drag point against the top-bar volume meter.
    ///
    /// The x-position is clamped to the meter width so dragging beyond the
    /// edges still emits a stable `SetVolume` action.
    pub(crate) fn top_bar_volume_drag_action(
        &self,
        layout: &ShellLayout,
        point: Point,
    ) -> Option<UiAction> {
        let controls = top_bar_controls_layout(layout, style_for_layout(layout).sizing);
        if !controls.active {
            return None;
        }
        Some(volume_action_for_meter(controls.volume_meter, point))
    }

    /// Resolve a map-point click to a sample-id action when map tab is active.
    pub(crate) fn map_sample_action_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        if !model.map.active {
            return None;
        }
        map_sample_id_at_point(layout, model, point)
            .map(|sample_id| UiAction::FocusMapSample { sample_id })
    }

    /// Resolve a modal confirm prompt button click into confirm/cancel actions.
    pub(crate) fn prompt_action_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        if !model.confirm_prompt.visible {
            return None;
        }
        let style = style_for_layout(layout);
        let (confirm_button, cancel_button) = prompt_buttons(layout, &style);
        if confirm_button.contains(point) {
            if prompt_has_validation_error(model) {
                return None;
            }
            return Some(UiAction::ConfirmPrompt);
        }
        if cancel_button.contains(point) {
            return Some(UiAction::CancelPrompt);
        }
        None
    }

    /// Return whether a point falls inside the active prompt text input rect.
    pub(crate) fn prompt_input_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        if !model.confirm_prompt.visible {
            return false;
        }
        let style = style_for_layout(layout);
        prompt_input_rect(layout, &style, model).is_some_and(|rect| rect.contains(point))
    }

    /// Return whether a point falls inside the waveform BPM text-input widget.
    pub(crate) fn waveform_bpm_input_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        let motion_model = NativeMotionModel::from_app_model(model);
        self.waveform_bpm_input_at_point_with_motion(layout, &motion_model, point)
    }

    /// Return whether a point falls inside the waveform BPM text-input widget.
    pub(crate) fn waveform_bpm_input_at_point_with_motion(
        &mut self,
        layout: &ShellLayout,
        motion_model: &NativeMotionModel,
        point: Point,
    ) -> bool {
        let style = style_for_layout(layout);
        let hit = self
            .cached_waveform_toolbar_buttons(layout, &style, motion_model)
            .iter()
            .any(|button| {
                button.label == "BPM Value" && button.enabled && button.rect.contains(point)
            });
        if hit {
            self.trigger_waveform_toolbar_flash(WaveformToolbarHoverHint::BpmValue);
        }
        hit
    }

    /// Return the waveform BPM input rect when the toolbar is available.
    pub(crate) fn waveform_bpm_input_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let motion_model = NativeMotionModel::from_app_model(model);
        let style = style_for_layout(layout);
        self.cached_waveform_toolbar_buttons(layout, &style, &motion_model)
            .iter()
            .find(|button| button.label == "BPM Value" && button.enabled)
            .map(|button| button.rect)
    }

    /// Return the waveform BPM text rect used for rendering inside the field.
    pub(crate) fn waveform_bpm_text_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let motion_model = NativeMotionModel::from_app_model(model);
        let style = style_for_layout(layout);
        self.cached_waveform_toolbar_buttons(layout, &style, &motion_model)
            .iter()
            .find(|button| button.label == "BPM Value" && button.enabled)
            .map(|button| compute_action_button_text_rect(button.rect, style.sizing))
    }

    /// Return the browser-search field rect when the toolbar is available.
    pub(crate) fn browser_search_field_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let (_, _, toolbar) = self.cached_browser_action_hit_test(layout, &style, model);
        (toolbar.search_field.width() > 1.0).then_some(toolbar.search_field)
    }

    #[cfg(test)]
    /// Return one browser rating-filter chip rect for the given signed level.
    pub(crate) fn browser_rating_filter_chip_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        level: i8,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let (_, _, toolbar) = self.cached_browser_action_hit_test(layout, &style, model);
        let index = browser_rating_filter_chip_index(level)?;
        let rect = toolbar.rating_filter_chips[index];
        (rect.width() > 1.0).then_some(rect)
    }

    /// Return the browser-search text rect used for rendering inside the field.
    pub(crate) fn browser_search_text_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let (_, _, toolbar) = self.cached_browser_action_hit_test(layout, &style, model);
        if toolbar.search_field.width() <= 1.0 {
            return None;
        }
        let toolbar_text_layout = compute_browser_toolbar_text_layout(
            toolbar.search_field,
            toolbar.activity_chip,
            toolbar.sort_chip,
            style.sizing,
        );
        Some(toolbar_text_layout.search_label)
    }

    /// Resolve a progress-overlay cancel click.
    pub(crate) fn progress_action_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        if !model.progress_overlay.visible
            || !model.progress_overlay.modal
            || !model.progress_overlay.cancelable
            || model.progress_overlay.cancel_requested
        {
            return None;
        }
        let style = style_for_layout(layout);
        progress_cancel_button(layout, &style, model.progress_overlay.modal)
            .contains(point)
            .then_some(UiAction::CancelProgress)
    }

    /// Build a native frame from state + layout + style tokens.
    #[cfg(test)]
    pub(crate) fn build_frame_with_style(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
    ) -> NativeViewFrame {
        let mut frame = NativeViewFrame {
            clear_color: style.clear_color,
            primitives: Vec::new(),
            text_runs: Vec::new(),
        };
        self.build_frame_with_style_into(layout, style, model, &mut frame);
        frame
    }

    /// Build a native frame from state + layout + style tokens into reusable buffers.
    #[cfg(test)]
    pub(crate) fn build_frame_with_style_into(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
        frame: &mut NativeViewFrame,
    ) {
        self.build_frame_with_style_into_with_motion(
            layout,
            style,
            model,
            frame,
            self.pulse_phase,
            true,
        );
    }

    /// Build a frame without animated values into reusable buffers.
    pub(crate) fn build_frame_with_style_into_static(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
        frame: &mut NativeViewFrame,
    ) {
        self.build_frame_with_style_into_with_motion(layout, style, model, frame, 0.0, false);
    }

    /// Build one static segment bucket into reusable buffers.
    pub(crate) fn build_static_segment_with_style_into(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
        motion_model: Option<&NativeMotionModel>,
        segment: StaticFrameSegment,
        segments: &mut StaticFrameSegments,
    ) {
        {
            let frame = segments.frame_mut(segment);
            frame.clear_color = style.clear_color;
            frame.primitives.clear();
            frame.text_runs.clear();
        }
        let emit_context = RefCell::new(SegmentedStaticEmitContext {
            layout,
            model,
            segments,
            target_segment: Some(segment),
        });
        let mut primitives = SegmentedPrimitiveSink {
            context: &emit_context,
        };
        let mut text_runs = SegmentedTextRunSink {
            context: &emit_context,
        };
        self.build_frame_with_style_into_with_motion_sinks(
            layout,
            style,
            model,
            &mut primitives,
            &mut text_runs,
            0.0,
            false,
            motion_model,
            Some(segment),
        );
    }

    /// Build a frame with a caller-supplied motion phase.
    fn build_frame_with_style_into_with_motion(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
        frame: &mut NativeViewFrame,
        pulse_phase: f32,
        include_overlays: bool,
    ) {
        frame.clear_color = style.clear_color;
        frame.primitives.clear();
        frame.text_runs.clear();
        self.build_frame_with_style_into_with_motion_sinks(
            layout,
            style,
            model,
            &mut frame.primitives,
            &mut frame.text_runs,
            pulse_phase,
            include_overlays,
            None,
            None,
        );
    }

    /// Build only waveform cursor/playhead motion overlays into reusable buffers.
    pub(crate) fn build_waveform_motion_overlay_into(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &NativeMotionModel,
        frame: &mut NativeViewFrame,
    ) {
        let sizing = style.sizing;
        frame.primitives.clear();
        frame.text_runs.clear();
        let primitives = &mut frame.primitives;
        let playhead_trail_lines = self.update_playhead_trail(layout.waveform_plot, model);
        push_waveform_playhead_overlay(
            primitives,
            layout,
            style,
            model,
            &playhead_trail_lines,
            self.hovered_waveform_resize_edge,
        );
        if let Some(hover_x) = self.waveform_hover_x {
            // Keep hover preview cursor visually obvious against dense waveform content.
            let hover_marker_width = (sizing.border_width * 2.0).max(2.0);
            if let Some(rect) =
                waveform_hover_marker_rect(layout.waveform_plot, hover_marker_width, hover_x)
            {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect,
                        color: blend_color(style.accent_warning, style.text_primary, 0.72),
                    }),
                );
                push_border(
                    primitives,
                    rect,
                    blend_color(style.accent_warning, style.text_primary, 0.48),
                    sizing.border_width,
                );
            }
        }
        frame.clear_color = style.clear_color;
    }

    /// Update retained playhead-trail samples and return drawable ghost lines.
    fn update_playhead_trail(
        &mut self,
        waveform_plot: Rect,
        model: &NativeMotionModel,
    ) -> Vec<PlayheadTrailLine> {
        let now_seconds = self.playhead_trail_elapsed_seconds;
        let previous = self.last_waveform_playhead_micros;
        let current = Self::playhead_position_micros(model);
        let view_window = (
            model.waveform_view_start_micros,
            model.waveform_view_end_micros,
        );
        let view_changed = self.last_waveform_view_window.replace(view_window) != Some(view_window);
        self.last_waveform_playhead_micros = current;
        if current.is_none() {
            self.playhead_trail_samples.clear();
            return Vec::new();
        }
        if !model.transport_running {
            self.playhead_trail_samples.clear();
            return Vec::new();
        }
        if view_changed {
            self.playhead_trail_samples.clear();
            return Vec::new();
        }
        self.append_playhead_trail_samples_if_moving(
            waveform_plot,
            true,
            previous,
            current,
            now_seconds,
        );
        self.prune_playhead_trail_samples(now_seconds);
        self.playhead_trail_lines(now_seconds)
    }

    /// Resolve normalized playhead position using micro precision when available.
    fn playhead_position_micros(model: &NativeMotionModel) -> Option<u32> {
        model.waveform_playhead_micros.or_else(|| {
            model
                .waveform_playhead_milli
                .map(|milli| u32::from(milli) * 1000)
        })
    }

    /// Return wrapped playhead delta in micro-units for forward/backward motion.
    fn wrapped_playhead_delta_micros(previous: u32, current: u32) -> i64 {
        let raw_delta = i64::from(current) - i64::from(previous);
        if raw_delta.abs() > 500_000 {
            if raw_delta > 0 {
                raw_delta - 1_000_000
            } else {
                raw_delta + 1_000_000
            }
        } else {
            raw_delta
        }
    }

    /// Insert one trail sample sequence for the latest frame when the playhead moved.
    fn append_playhead_trail_samples_if_moving(
        &mut self,
        waveform_plot: Rect,
        transport_running: bool,
        previous: Option<u32>,
        current: Option<u32>,
        captured_at_seconds: f32,
    ) {
        if !transport_running {
            return;
        }
        let (Some(previous), Some(current)) = (previous, current) else {
            return;
        };
        let delta = Self::wrapped_playhead_delta_micros(previous, current);
        if delta == 0 {
            return;
        }
        if delta.unsigned_abs() > PLAYHEAD_TRAIL_MAX_CONTIGUOUS_DELTA_MICROS {
            self.playhead_trail_samples.clear();
            return;
        }
        let previous_ratio = previous as f32 / 1_000_000.0;
        let current_ratio = Self::unwrap_playhead_ratio(previous_ratio, current, delta);
        let delta_ratio = current_ratio - previous_ratio;
        let previous_capture_seconds = self
            .playhead_trail_samples
            .last()
            .map(|sample| sample.captured_at_seconds)
            .unwrap_or(captured_at_seconds - PLAYHEAD_TRAIL_MIN_INTERPOLATED_DELTA_SECONDS);
        let capture_delta_seconds = (captured_at_seconds - previous_capture_seconds)
            .max(PLAYHEAD_TRAIL_MIN_INTERPOLATED_DELTA_SECONDS);
        let pixel_step_ratio = (0.5 / waveform_plot.width().max(1.0)).clamp(0.00025, 0.02);
        let steps_by_pixel = (delta_ratio.abs() / pixel_step_ratio).ceil() as usize;
        let steps_by_time =
            (capture_delta_seconds / PLAYHEAD_TRAIL_MIN_INTERPOLATED_DELTA_SECONDS).ceil() as usize;
        let steps = steps_by_pixel
            .max(steps_by_time)
            .clamp(1, PLAYHEAD_TRAIL_MAX_INTERPOLATED_STEPS);
        for step in 1..=steps {
            let progress = step as f32 / steps as f32;
            let ratio = (previous_ratio + (delta_ratio * progress)).rem_euclid(1.0);
            self.playhead_trail_samples.push(PlayheadTrailSample {
                ratio,
                captured_at_seconds: previous_capture_seconds + (capture_delta_seconds * progress),
            });
        }
    }

    /// Convert wrapped playhead movement to an unwrapped normalized ratio.
    fn unwrap_playhead_ratio(previous_ratio: f32, current_micros: u32, delta_micros: i64) -> f32 {
        let mut current_ratio = current_micros as f32 / 1_000_000.0;
        if delta_micros > 0 && current_ratio < previous_ratio {
            current_ratio += 1.0;
        } else if delta_micros < 0 && current_ratio > previous_ratio {
            current_ratio -= 1.0;
        }
        current_ratio
    }

    /// Remove expired and overflowed trail samples from retained state.
    fn prune_playhead_trail_samples(&mut self, now_seconds: f32) {
        self.playhead_trail_samples.retain(|sample| {
            (now_seconds - sample.captured_at_seconds).max(0.0) <= PLAYHEAD_TRAIL_FADE_SECONDS
        });
        let overflow = self
            .playhead_trail_samples
            .len()
            .saturating_sub(PLAYHEAD_TRAIL_MAX_SAMPLES);
        if overflow > 0 {
            self.playhead_trail_samples.drain(0..overflow);
        }
    }

    /// Project retained trail samples into drawable ghost-line primitives.
    ///
    /// Alpha is normalized across the currently retained trail so fast motion still renders
    /// a full head-to-tail fade instead of large equal-opacity slabs, while the trail itself
    /// starts below the fully opaque playhead marker.
    fn playhead_trail_lines(&self, now_seconds: f32) -> Vec<PlayheadTrailLine> {
        let retained = self
            .playhead_trail_samples
            .iter()
            .filter_map(|sample| {
                let age_seconds = (now_seconds - sample.captured_at_seconds)
                    .clamp(0.0, PLAYHEAD_TRAIL_FADE_SECONDS);
                (age_seconds < PLAYHEAD_TRAIL_FADE_SECONDS).then_some(*sample)
            })
            .collect::<Vec<_>>();
        let last_index = retained.len().saturating_sub(1).max(1) as f32;
        retained
            .into_iter()
            .enumerate()
            .filter_map(|(index, sample)| {
                let progress = index as f32 / last_index;
                let alpha = (progress.powf(1.35) * PLAYHEAD_TRAIL_HEAD_ALPHA).clamp(0.0, 1.0);
                (alpha > 0.01).then_some(PlayheadTrailLine {
                    ratio: sample.ratio,
                    alpha,
                })
            })
            .collect()
    }

    /// Build only heavier motion-driven chrome overlays into reusable buffers.
    pub(crate) fn build_chrome_motion_overlay_into(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &NativeMotionModel,
        frame: &mut NativeViewFrame,
    ) {
        let sizing = style.sizing;
        let motion_wave = interaction_wave(self.pulse_phase);
        frame.primitives.clear();
        frame.text_runs.clear();
        let primitives = &mut frame.primitives;
        let text_runs = &mut frame.text_runs;

        let waveform_toolbar_buttons = waveform_toolbar_buttons(
            layout,
            style,
            model,
            self.waveform_bpm_input_active,
            self.waveform_bpm_input_display.as_deref(),
        );
        let waveform_toolbar_left = waveform_toolbar_left_edge(
            &waveform_toolbar_buttons,
            layout.waveform_header.max.x - sizing.text_inset_x,
        );
        push_waveform_header_overlay(
            primitives,
            text_runs,
            layout,
            style,
            model,
            Some(waveform_toolbar_left - sizing.action_button_gap),
        );
        if self.waveform_bpm_input_active {
            if let Some(bpm_input_rect) = waveform_toolbar_buttons
                .iter()
                .find(|button| button.label == "BPM Value")
                .map(|button| button.rect)
            {
                if let Some(visual) = self.waveform_bpm_editor_visual.as_ref() {
                    let bpm_text_rect = waveform_toolbar_buttons
                        .iter()
                        .find(|button| button.label == "BPM Value")
                        .map(|button| compute_action_button_text_rect(button.rect, sizing))
                        .unwrap_or(bpm_input_rect);
                    render_active_waveform_bpm_editor(
                        primitives,
                        text_runs,
                        style,
                        sizing,
                        bpm_input_rect,
                        bpm_text_rect,
                        visual,
                    );
                } else {
                    render_waveform_bpm_input_focus_overlay(
                        primitives,
                        style,
                        sizing,
                        bpm_input_rect,
                        motion_wave,
                    );
                }
            }
        }
        render_waveform_toolbar_buttons(
            primitives,
            text_runs,
            style,
            sizing,
            &waveform_toolbar_buttons,
            self.hovered_waveform_toolbar_hint,
            self.waveform_toolbar_flash.map(|flash| flash.hint),
            motion_wave,
            self.waveform_bpm_editor_visual.is_some(),
        );
        if let Some(search_field_rect) = self
            .browser_toolbar_layout
            .as_ref()
            .map(|toolbar| toolbar.search_field)
            .filter(|rect| rect.width() > 1.0)
        {
            if self.hovered_browser_search_field && self.browser_search_editor_visual.is_none() {
                render_browser_search_field_hover_overlay(
                    primitives,
                    style,
                    sizing,
                    search_field_rect,
                    motion_wave,
                );
            }
        }
        if let Some((chip_rect, rating_level)) =
            self.browser_toolbar_layout.as_ref().and_then(|toolbar| {
                let hovered_level = self.hovered_browser_rating_filter_level?;
                let index = browser_rating_filter_chip_index(hovered_level)?;
                let chip_rect = toolbar.rating_filter_chips[index];
                (chip_rect.width() > 1.0).then_some((chip_rect, hovered_level))
            })
        {
            let active = browser_rating_filter_chip_index(rating_level)
                .and_then(|index| model.active_rating_filters.get(index))
                .copied()
                .unwrap_or(false);
            render_browser_rating_filter_chip_hover_overlay(
                primitives,
                style,
                sizing,
                chip_rect,
                rating_level,
                active,
                motion_wave,
            );
        }
        if let Some(button_rect) = source_add_button_rect(layout.sidebar_header, sizing) {
            let hovered = self.hovered_source_add_button;
            let flashed = self.source_add_button_flash_ticks > 0;
            if hovered || flashed {
                render_source_add_button_overlay(
                    primitives,
                    text_runs,
                    style,
                    sizing,
                    button_rect,
                    hovered,
                    flashed,
                    motion_wave,
                );
            }
        }
        if let Some(button_rect) = status_options_button_rect(layout.top_bar_action_cluster, sizing)
        {
            let hovered = self.hovered_status_options_button;
            let flashed = self.status_options_button_flash_ticks > 0;
            if hovered || flashed {
                render_status_options_button(
                    primitives,
                    style,
                    sizing,
                    button_rect,
                    hovered,
                    flashed,
                    motion_wave,
                );
            }
        }

        let tabs = compute_browser_tabs_rects(layout.browser_tabs, sizing);
        let (samples_fill, map_fill) = if !model.map_active {
            (
                blend_color(
                    style.surface_overlay,
                    style.bg_tertiary,
                    style.state_selected_blend + (motion_wave * 0.1),
                ),
                style.surface_base,
            )
        } else {
            (
                style.surface_base,
                blend_color(
                    style.surface_overlay,
                    style.bg_tertiary,
                    style.state_selected_blend + (motion_wave * 0.1),
                ),
            )
        };
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: tabs.samples,
                color: samples_fill,
            }),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: tabs.map,
                color: map_fill,
            }),
        );
        push_border(primitives, tabs.samples, style.border, sizing.border_width);
        push_border(
            primitives,
            tabs.map,
            blend_color(style.accent_mint, style.text_primary, 0.42),
            sizing.border_width,
        );
        Self::push_status_right_motion_overlay(
            primitives,
            text_runs,
            layout,
            style,
            &model.status_right,
            None,
        );

        frame.clear_color = style.clear_color;
    }

    /// Build all motion-sensitive overlays into one reusable buffer.
    #[cfg(test)]
    pub(crate) fn build_motion_overlay_into(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &NativeMotionModel,
        frame: &mut NativeViewFrame,
    ) {
        self.build_waveform_motion_overlay_into(layout, style, model, frame);
        let mut chrome_frame = NativeViewFrame {
            clear_color: style.clear_color,
            primitives: Vec::new(),
            text_runs: Vec::new(),
        };
        self.build_chrome_motion_overlay_into(layout, style, model, &mut chrome_frame);
        frame.primitives.extend(chrome_frame.primitives);
        frame.text_runs.extend(chrome_frame.text_runs);
        frame.clear_color = style.clear_color;
    }

    /// Build a native frame using default style tokens.
    #[cfg(test)]
    pub(crate) fn build_frame(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> NativeViewFrame {
        self.build_frame_with_style(layout, &style_for_layout(layout), model)
    }

    fn push_status_right_motion_overlay(
        primitives: &mut impl PrimitiveSink,
        text_runs: &mut impl TextRunSink,
        layout: &ShellLayout,
        style: &StyleTokens,
        status_right: &str,
        options_button_rect: Option<Rect>,
    ) {
        if status_right.is_empty() {
            return;
        }
        let sizing = style.sizing;
        let text_segment = if let Some(button_rect) = options_button_rect {
            Rect::from_min_max(
                layout.status_right_segment.min,
                Point::new(
                    (button_rect.min.x - sizing.text_inset_x.max(3.0))
                        .max(layout.status_right_segment.min.x),
                    layout.status_right_segment.max.y,
                ),
            )
        } else {
            layout.status_right_segment
        };
        let background_rect = status_motion_overlay_rect(text_segment, sizing.border_width);
        if background_rect.width() > 0.0 && background_rect.height() > 0.0 {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: background_rect,
                    color: style.surface_raised,
                }),
            );
        }
        let status_text_rect =
            status_right_text_rect(layout.status_right_segment, sizing, options_button_rect);
        emit_text(
            text_runs,
            TextRun {
                text: truncate_to_width(
                    status_right,
                    status_text_rect.width().max(36.0),
                    sizing.font_status,
                ),
                position: status_text_rect.min,
                font_size: sizing.font_status,
                color: style.text_muted,
                max_width: Some(status_text_rect.width().max(36.0)),
                align: TextAlign::Right,
            },
        );
    }
}

fn status_motion_overlay_rect(segment: Rect, stroke: f32) -> Rect {
    let inset = stroke.max(1.0);
    let min = Point::new(
        (segment.min.x + inset).min(segment.max.x),
        (segment.min.y + inset).min(segment.max.y),
    );
    let max = Point::new(
        (segment.max.x - inset).max(min.x),
        (segment.max.y - inset).max(min.y),
    );
    Rect::from_min_max(min, max)
}

#[cfg(test)]
mod tests;
