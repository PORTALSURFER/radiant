//! Mutable interaction state and paint generation for the native shell.

use super::{
    layout::{ShellLayout, ShellNodeKind},
    layout_adapter::{
        BrowserTabsRects, SidebarRowCounts, compute_action_button_text_rect,
        compute_browser_action_button_rects, compute_browser_footer_text_rect,
        compute_browser_header_text_layout, compute_browser_map_canvas_rect,
        compute_browser_map_header_text_layout, compute_browser_map_point_center,
        compute_browser_row_text_layout, compute_browser_tabs_rects,
        compute_browser_tabs_text_layout, compute_browser_toolbar_sections,
        compute_browser_toolbar_text_layout, compute_drag_overlay_text_layout,
        compute_drag_overlay_visual_layout, compute_progress_overlay_text_layout,
        compute_progress_overlay_visual_layout, compute_prompt_overlay_text_layout,
        compute_prompt_overlay_visual_layout, compute_row_index_at_point,
        compute_sidebar_action_button_rects, compute_sidebar_folder_header_layout,
        compute_sidebar_folder_row_text_rect, compute_sidebar_footer_text_layout,
        compute_sidebar_header_text_layout, compute_sidebar_recovery_badge_text_rect,
        compute_sidebar_row_sections, compute_sidebar_source_row_text_rect,
        compute_source_section_divider_rect, compute_status_text_line_rect,
        compute_top_bar_controls_sections, compute_top_bar_controls_text_layout,
        compute_top_bar_title_text_rect, compute_top_bar_update_text_layout,
        compute_update_action_button_rects, compute_waveform_annotation_rects,
        compute_waveform_header_text_layout,
    },
    paint::{FillCircle, FillRect, NativeViewFrame, Primitive, TextAlign, TextRun},
    style::{SizingTokens, StyleTokens},
};
use crate::app::{
    AppModel, BrowserRowModel, BrowserTagTarget, DirtySegments, NativeMotionModel, UiAction,
};
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
mod overlays;
mod waveform_segments;

use self::{browser_rows::*, overlays::*, waveform_segments::*};

/// Maximum retained entries for browser-row text truncation outputs.
const BROWSER_ROW_TRUNCATION_CACHE_CAPACITY: usize = 1024;

/// Mutable interaction + animation state for the native shell.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NativeShellState {
    selected_column: usize,
    hovered: Option<ShellNodeKind>,
    hovered_browser_visible_row: Option<usize>,
    waveform_hover_x: Option<f32>,
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
    browser_rows_cache_key: Option<BrowserRowsCacheKey>,
    browser_row_truncation_cache: BrowserRowTruncationCache,
    browser_row_truncation_cache_key: Option<BrowserRowTruncationCacheKey>,
    browser_row_truncation_frame_counts: BrowserRowTruncationFrameCounts,
}

/// Per-build browser-row truncation cache lookup counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct BrowserRowTruncationFrameCounts {
    /// Number of truncation lookups requested while building browser rows.
    pub lookup_count: u32,
    /// Number of lookups that reused cached truncated strings.
    pub cache_hit_count: u32,
    /// Number of lookups that required fresh truncation work.
    pub cache_miss_count: u32,
}

/// Browser row text variants tracked in truncation cache keys.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum BrowserRowTextKind {
    /// Primary sample label text in browser rows.
    Sample,
    /// Secondary bucket/chip label text in browser rows.
    Bucket,
}

/// Lookup key for one browser-row truncation output.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct BrowserRowTruncationEntryKey {
    /// Stable visible-row identity used to scope cached text.
    row_id: u32,
    /// Quantized width bucket used by truncation heuristics.
    width_bucket: u16,
    /// Quantized font-size bucket used by truncation heuristics.
    font_size_bucket: u16,
    /// Distinguishes sample-label vs bucket-label truncation outputs.
    text_kind: BrowserRowTextKind,
}

/// Invalidation key for browser-row truncation cache content.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct BrowserRowTruncationCacheKey {
    /// Browser rows region minimum x-coordinate.
    browser_rows_min_x: u32,
    /// Browser rows region minimum y-coordinate.
    browser_rows_min_y: u32,
    /// Browser rows region maximum x-coordinate.
    browser_rows_max_x: u32,
    /// Browser rows region maximum y-coordinate.
    browser_rows_max_y: u32,
    /// Sample-label font size token bits.
    font_body_bits: u32,
    /// Bucket-label font size token bits.
    font_meta_bits: u32,
    /// Effective UI scale token bits.
    ui_scale: u32,
    /// Visible-window row-label content revision fingerprint.
    row_text_revision: u64,
}

/// Small retained LRU cache for browser-row text truncation outputs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct BrowserRowTruncationCache {
    values: HashMap<BrowserRowTruncationEntryKey, BrowserRowTruncationCacheValue>,
    touch_epoch: u64,
}

/// One cached truncation result with the latest logical access epoch.
#[derive(Clone, Debug, PartialEq, Eq)]
struct BrowserRowTruncationCacheValue {
    truncated: String,
    last_touch_epoch: u64,
}

/// Ephemeral sidebar source-menu state tracked by the runtime.
#[derive(Clone, Copy, Debug, PartialEq)]
struct SourceContextMenuState {
    /// Source row index the menu actions target.
    row_index: usize,
    /// Pointer anchor used to place the floating menu panel.
    anchor: Point,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct NativeAnimationReasons {
    transport_running: bool,
    startup_frame_tick: bool,
}

/// Cursor-move effect classification used by runtime overlay invalidation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CursorMoveEffect {
    /// Pointer movement did not change observable hover state.
    None,
    /// Only waveform hover-cursor position changed.
    WaveformHoverOnly,
    /// Hovered node and/or hovered row changed.
    GeneralOverlay,
}

/// Compact state-overlay fingerprint for change detection in runtime caches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StateOverlayFingerprint {
    /// Selected browser column index.
    pub selected_column: usize,
    /// Current hovered shell node kind.
    pub hovered: Option<ShellNodeKind>,
    /// Hovered browser row in visible-row space.
    pub hovered_browser_visible_row: Option<usize>,
    /// Whether focused selection emphasis is active.
    pub has_focus_emphasis: bool,
}

/// Compact motion-overlay fingerprint for runtime overlay skip checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MotionOverlayFingerprint {
    /// Whether transport-running animation is active.
    pub transport_running: bool,
    /// Remaining startup animation ticks.
    pub startup_frame_ticks: u8,
    /// Quantized pulse animation phase.
    pub pulse_phase_bits: u32,
    /// Hovered waveform marker x-position bits in shell-space coordinates.
    pub waveform_hover_x_bits: Option<u32>,
}

/// Static-scene segments used for retained incremental scene composition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum StaticFrameSegment {
    /// Status-bar text and chrome.
    StatusBar,
    /// Browser metadata/chrome outside row-window and map canvas.
    BrowserFrame,
    /// Browser row-window list content.
    BrowserRowsWindow,
    /// Browser map panel content and map-header details.
    MapPanel,
    /// Waveform panel/chrome static content.
    WaveformOverlay,
    /// Remaining static content outside explicit segment buckets.
    GlobalStatic,
}

impl StaticFrameSegment {
    /// Number of static segment buckets.
    pub(crate) const COUNT: usize = 6;

    /// Deterministic segment iteration order for scene composition.
    pub(crate) const ALL: [Self; Self::COUNT] = [
        Self::GlobalStatic,
        Self::WaveformOverlay,
        Self::BrowserRowsWindow,
        Self::MapPanel,
        Self::BrowserFrame,
        Self::StatusBar,
    ];

    /// Return the segment index for cache arrays.
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::GlobalStatic => 0,
            Self::WaveformOverlay => 1,
            Self::BrowserFrame => 2,
            Self::BrowserRowsWindow => 3,
            Self::MapPanel => 4,
            Self::StatusBar => 5,
        }
    }

    /// Return the corresponding bridge dirty-segment bit.
    pub(crate) const fn dirty_mask(self) -> u16 {
        match self {
            Self::StatusBar => DirtySegments::STATUS_BAR,
            Self::BrowserFrame => DirtySegments::BROWSER_FRAME,
            Self::BrowserRowsWindow => DirtySegments::BROWSER_ROWS_WINDOW,
            Self::MapPanel => DirtySegments::MAP_PANEL,
            Self::WaveformOverlay => DirtySegments::WAVEFORM_OVERLAY,
            Self::GlobalStatic => DirtySegments::GLOBAL_STATIC,
        }
    }
}

/// Static scene fragments split into deterministic segment buckets.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StaticFrameSegments {
    frames: [NativeViewFrame; StaticFrameSegment::COUNT],
}

impl Default for StaticFrameSegments {
    /// Create empty frame buckets for each static segment.
    fn default() -> Self {
        Self {
            frames: std::array::from_fn(|_| NativeViewFrame::default()),
        }
    }
}

impl StaticFrameSegments {
    /// Return an immutable frame buffer for one static segment.
    pub(crate) fn frame(&self, segment: StaticFrameSegment) -> &NativeViewFrame {
        &self.frames[segment.index()]
    }

    /// Return a mutable frame buffer for one static segment.
    pub(crate) fn frame_mut(&mut self, segment: StaticFrameSegment) -> &mut NativeViewFrame {
        &mut self.frames[segment.index()]
    }

    /// Compose all static segments into one full static frame.
    pub(crate) fn compose_into(&self, frame: &mut NativeViewFrame) {
        frame.primitives.clear();
        frame.text_runs.clear();
        for segment in StaticFrameSegment::ALL {
            let segment_frame = self.frame(segment);
            frame.clear_color = segment_frame.clear_color;
            frame
                .primitives
                .extend(segment_frame.primitives.iter().copied());
            frame
                .text_runs
                .extend(segment_frame.text_runs.iter().cloned());
        }
    }
}

/// Sink for emitted frame primitives.
trait PrimitiveSink {
    /// Push one primitive into the sink.
    fn push_primitive(&mut self, primitive: Primitive);
}

impl PrimitiveSink for Vec<Primitive> {
    fn push_primitive(&mut self, primitive: Primitive) {
        self.push(primitive);
    }
}

/// Sink for emitted frame text runs.
trait TextRunSink {
    /// Push one text run into the sink.
    fn push_text_run(&mut self, text_run: TextRun);
}

impl TextRunSink for Vec<TextRun> {
    fn push_text_run(&mut self, text_run: TextRun) {
        self.push(text_run);
    }
}

/// Emit one primitive into a generic sink.
fn emit_primitive(primitives: &mut impl PrimitiveSink, primitive: Primitive) {
    primitives.push_primitive(primitive);
}

/// Emit one text run into a generic sink.
fn emit_text(text_runs: &mut impl TextRunSink, text_run: TextRun) {
    text_runs.push_text_run(text_run);
}

/// Shared segmented emit context that routes output into static buckets.
struct SegmentedStaticEmitContext<'a> {
    layout: &'a ShellLayout,
    model: &'a AppModel,
    segments: &'a mut StaticFrameSegments,
    target_segment: Option<StaticFrameSegment>,
}

/// Primitive sink that routes primitives directly into static buckets.
struct SegmentedPrimitiveSink<'a, 'b> {
    context: &'a RefCell<SegmentedStaticEmitContext<'b>>,
}

impl PrimitiveSink for SegmentedPrimitiveSink<'_, '_> {
    fn push_primitive(&mut self, primitive: Primitive) {
        let mut context = self.context.borrow_mut();
        let segment = static_segment_for_primitive(context.layout, context.model, &primitive);
        if context
            .target_segment
            .is_some_and(|target| target != segment)
        {
            return;
        }
        context
            .segments
            .frame_mut(segment)
            .primitives
            .push(primitive);
    }
}

/// Text-run sink that routes text directly into static buckets.
struct SegmentedTextRunSink<'a, 'b> {
    context: &'a RefCell<SegmentedStaticEmitContext<'b>>,
}

impl TextRunSink for SegmentedTextRunSink<'_, '_> {
    fn push_text_run(&mut self, text_run: TextRun) {
        let mut context = self.context.borrow_mut();
        let segment = static_segment_for_text(context.layout, context.model, &text_run);
        if context
            .target_segment
            .is_some_and(|target| target != segment)
        {
            return;
        }
        context.segments.frame_mut(segment).text_runs.push(text_run);
    }
}

impl NativeAnimationReasons {
    fn needs_animation(self) -> bool {
        self.transport_running || self.startup_frame_tick
    }
}

impl BrowserRowTruncationCache {
    /// Clear all retained truncation entries.
    fn clear(&mut self) {
        self.values.clear();
        self.touch_epoch = 0;
    }

    /// Resolve one truncation output from cache or compute and insert on miss.
    fn resolve(
        &mut self,
        key: BrowserRowTruncationEntryKey,
        text: &str,
        max_width: f32,
        font_size: f32,
        frame_counts: &mut BrowserRowTruncationFrameCounts,
    ) -> String {
        let touch_epoch = self.next_touch_epoch();
        frame_counts.lookup_count = frame_counts.lookup_count.saturating_add(1);
        if let Some(cached) = self.values.get_mut(&key) {
            frame_counts.cache_hit_count = frame_counts.cache_hit_count.saturating_add(1);
            cached.last_touch_epoch = touch_epoch;
            return cached.truncated.clone();
        }
        frame_counts.cache_miss_count = frame_counts.cache_miss_count.saturating_add(1);
        let truncated = truncate_to_width(text, max_width, font_size);
        self.insert(key, truncated.clone(), touch_epoch);
        truncated
    }

    /// Return the next logical access epoch used for cache aging.
    fn next_touch_epoch(&mut self) -> u64 {
        if self.touch_epoch == u64::MAX {
            // This epoch only grows during one process lifetime; clear on overflow.
            self.clear();
        }
        self.touch_epoch = self.touch_epoch.saturating_add(1);
        self.touch_epoch
    }

    /// Insert one key/value pair and enforce the fixed cache capacity via LRU epoch eviction.
    fn insert(&mut self, key: BrowserRowTruncationEntryKey, value: String, touch_epoch: u64) {
        self.values.insert(
            key,
            BrowserRowTruncationCacheValue {
                truncated: value,
                last_touch_epoch: touch_epoch,
            },
        );
        while self.values.len() > BROWSER_ROW_TRUNCATION_CACHE_CAPACITY {
            let Some((evicted, _)) = self
                .values
                .iter()
                .min_by_key(|(_, value)| value.last_touch_epoch)
                .map(|(key, value)| (*key, value.last_touch_epoch))
            else {
                break;
            };
            self.values.remove(&evicted);
        }
    }
}

impl NativeShellState {
    /// Create a default shell state.
    pub(crate) fn new() -> Self {
        Self {
            selected_column: 1,
            hovered: None,
            hovered_browser_visible_row: None,
            waveform_hover_x: None,
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
            browser_rows_cache_key: None,
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
        }
    }

    /// Return whether playback transport is currently reported as running.
    pub(crate) fn is_transport_running(&self) -> bool {
        self.transport_running
    }

    /// Synchronize local interaction state from the latest app model.
    pub(crate) fn sync_from_model(&mut self, model: &AppModel) {
        self.selected_column = model.selected_column.min(2);
        self.transport_running = model.transport_running;
        self.startup_frame_ticks = self.startup_frame_ticks.saturating_sub(1);
        if model.map.active {
            self.hovered_browser_visible_row = None;
        }
        if self
            .source_context_menu
            .is_some_and(|menu| menu.row_index >= model.sources.rows.len())
        {
            self.source_context_menu = None;
        }
        self.has_focus_emphasis = model
            .browser
            .rows
            .iter()
            .any(|row| row.focused || row.selected)
            || model.sources.rows.iter().any(|row| row.selected)
            || model
                .sources
                .folder_rows
                .iter()
                .any(|row| row.focused || row.selected)
            || model.confirm_prompt.visible;
    }

    /// Synchronize motion-sensitive state from a dedicated motion model projection.
    pub(crate) fn sync_from_motion_model(&mut self, model: &NativeMotionModel) {
        self.transport_running = model.transport_running;
    }

    /// Return the current state-overlay fingerprint.
    pub(crate) fn state_overlay_fingerprint(&self) -> StateOverlayFingerprint {
        StateOverlayFingerprint {
            selected_column: self.selected_column,
            hovered: self.hovered,
            hovered_browser_visible_row: self.hovered_browser_visible_row,
            has_focus_emphasis: self.has_focus_emphasis,
        }
    }

    /// Return the current motion-overlay fingerprint.
    pub(crate) fn motion_overlay_fingerprint(&self) -> MotionOverlayFingerprint {
        MotionOverlayFingerprint {
            transport_running: self.transport_running,
            startup_frame_ticks: self.startup_frame_ticks,
            pulse_phase_bits: self.pulse_phase.to_bits(),
            waveform_hover_x_bits: self.waveform_hover_x.map(f32::to_bits),
        }
    }

    /// Return browser-row truncation lookup counts from the latest row-cache refresh.
    #[cfg(test)]
    pub(crate) fn browser_row_truncation_frame_counts(&self) -> BrowserRowTruncationFrameCounts {
        self.browser_row_truncation_frame_counts
    }

    /// Update animation clocks by a frame delta using explicit style motion tokens.
    pub(crate) fn tick_with_style(&mut self, delta_seconds: f32, style: &StyleTokens) {
        if self.needs_animation() {
            let speed = if self.transport_running {
                style.motion_speed_transport
            } else {
                style.motion_speed_idle
            };
            self.pulse_phase =
                (self.pulse_phase + delta_seconds * speed).rem_euclid(std::f32::consts::TAU);
        }
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
        let next_waveform_hover_x = waveform_hover_x_for_point(layout, next_hover, point);
        let hover_changed = next_hover != self.hovered;
        let browser_row_changed = next_hovered_browser_row != self.hovered_browser_visible_row;
        let waveform_hover_changed =
            next_waveform_hover_x.map(f32::to_bits) != self.waveform_hover_x.map(f32::to_bits);
        if !hover_changed && !browser_row_changed && !waveform_hover_changed {
            return CursorMoveEffect::None;
        }
        self.hovered = next_hover;
        self.hovered_browser_visible_row = next_hovered_browser_row;
        self.waveform_hover_x = next_waveform_hover_x;
        if waveform_hover_changed && !hover_changed && !browser_row_changed {
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
            KeyCode::Enter => {
                self.transport_running = !self.transport_running;
                true
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
            .map(|button| button.action)
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

    /// Return a browser-action button rect for the provided action in tests.
    #[cfg(test)]
    pub(crate) fn browser_action_button_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        action: UiAction,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        browser_action_buttons(layout, &style, model)
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
        waveform_toolbar_buttons(layout, &style, &motion_model)
            .into_iter()
            .find(|button| button.label == label)
            .map(|button| button.rect)
    }

    /// Resolve a source-management action button click into a native UI action.
    pub(crate) fn source_action_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
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

    /// Resolve a browser action-strip click into a native UI action.
    pub(crate) fn browser_action_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        let buttons = browser_action_buttons(layout, &style, model);
        let toolbar = browser_toolbar_layout(layout, &style, &buttons);
        if toolbar.search_field.width() > 1.0 && toolbar.search_field.contains(point) {
            return Some(UiAction::FocusBrowserSearch);
        }
        if let Some(action) = browser_column_chips(layout, &style, model, &buttons)
            .into_iter()
            .find(|chip| chip.rect.contains(point))
            .map(|chip| UiAction::SelectColumn { index: chip.column })
        {
            return Some(action);
        }
        buttons
            .into_iter()
            .find(|button| button.enabled && button.rect.contains(point))
            .map(|button| button.action)
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
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        let motion_model = NativeMotionModel::from_app_model(model);
        waveform_toolbar_buttons(layout, &style, &motion_model)
            .into_iter()
            .find(|button| button.enabled && button.rect.contains(point))
            .and_then(|button| button.action)
    }

    /// Resolve a top-bar update action button click.
    pub(crate) fn update_action_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        update_action_buttons(layout, &style, model)
            .into_iter()
            .find(|button| button.enabled && button.rect.contains(point))
            .map(|button| button.action)
    }

    /// Resolve a click inside the top-bar options label to a native options action.
    pub(crate) fn top_bar_options_action_at_point(
        &self,
        layout: &ShellLayout,
        point: Point,
    ) -> Option<UiAction> {
        let controls = top_bar_controls_layout(layout, style_for_layout(layout).sizing);
        if !controls.active || !controls.options_label.contains(point) {
            return None;
        }
        Some(UiAction::OpenOptionsMenu)
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

    /// Resolve a progress-overlay cancel click.
    pub(crate) fn progress_action_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        if !model.progress_overlay.visible
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
        );
    }

    /// Build a frame with a caller-supplied motion phase into generic sinks.
    fn build_frame_with_style_into_with_motion_sinks(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
        primitives: &mut impl PrimitiveSink,
        text_runs: &mut impl TextRunSink,
        pulse_phase: f32,
        include_overlays: bool,
        static_segment_filter: Option<StaticFrameSegment>,
    ) {
        let sizing = style.sizing;
        let motion_wave = interaction_wave(pulse_phase);
        let focus_fill_emphasis = focus_fill_blend(style, motion_wave);
        let focus_text_emphasis = focus_text_blend(style, motion_wave);
        let build_global_static =
            static_segment_matches(static_segment_filter, StaticFrameSegment::GlobalStatic);
        let build_waveform_overlay =
            static_segment_matches(static_segment_filter, StaticFrameSegment::WaveformOverlay);
        let build_browser_rows_window =
            static_segment_matches(static_segment_filter, StaticFrameSegment::BrowserRowsWindow);
        let build_map_panel =
            static_segment_matches(static_segment_filter, StaticFrameSegment::MapPanel);
        let build_browser_frame =
            static_segment_matches(static_segment_filter, StaticFrameSegment::BrowserFrame);
        let build_status_bar =
            static_segment_matches(static_segment_filter, StaticFrameSegment::StatusBar);
        let build_browser_rows_or_map = build_browser_rows_window || build_map_panel;

        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: layout.top_bar,
                color: style.surface_raised,
            }),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: layout.sidebar,
                color: style.surface_raised,
            }),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: layout.content,
                color: style.surface_base,
            }),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: layout.waveform_card,
                color: style.surface_raised,
            }),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: layout.status_bar,
                color: style.surface_raised,
            }),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: layout.browser_panel,
                color: style.surface_raised,
            }),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: layout.browser_tabs,
                color: style.surface_overlay,
            }),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: layout.browser_toolbar,
                color: style.surface_base,
            }),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: layout.browser_table_header,
                color: style.surface_overlay,
            }),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: layout.browser_footer,
                color: style.surface_overlay,
            }),
        );

        let waveform_inner = layout.waveform_plot;
        if build_waveform_overlay {
            let scan_step = sizing.waveform_scan_step;
            let mut x = waveform_inner.min.x;
            while x < waveform_inner.max.x {
                let strong = ((x - waveform_inner.min.x) / scan_step).floor() as i32 % 4 == 0;
                let line_color = if strong {
                    style.grid_strong
                } else {
                    style.grid_soft
                };
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: Rect::from_min_max(
                            Point::new(x, waveform_inner.min.y),
                            Point::new(
                                (x + sizing.border_width).min(waveform_inner.max.x),
                                waveform_inner.max.y,
                            ),
                        ),
                        color: line_color,
                    }),
                );
                x += scan_step;
            }
            push_waveform_image(
                primitives,
                waveform_inner,
                model.waveform.waveform_image.as_deref(),
            );
            let motion_model = NativeMotionModel::from_app_model(model);
            let waveform_toolbar_buttons = waveform_toolbar_buttons(layout, style, &motion_model);
            let waveform_toolbar_left = waveform_toolbar_left_edge(
                &waveform_toolbar_buttons,
                layout.waveform_header.max.x - sizing.text_inset_x,
            );
            push_waveform_header_overlay(
                primitives,
                text_runs,
                layout,
                style,
                &motion_model,
                Some(waveform_toolbar_left - sizing.action_button_gap),
            );
            render_waveform_toolbar_buttons(
                primitives,
                text_runs,
                style,
                sizing,
                &waveform_toolbar_buttons,
            );
        }

        let browser_buttons = browser_action_buttons(layout, style, model);
        let browser_column_chips = browser_column_chips(layout, style, model, &browser_buttons);
        let source_row_rects = if build_global_static {
            rendered_source_row_rects(layout, style, model)
        } else {
            Vec::new()
        };
        let folder_row_rects = if build_global_static {
            rendered_folder_row_rects(layout, style, model)
        } else {
            Vec::new()
        };
        let browser_rows = if build_browser_rows_or_map {
            rendered_browser_rows(layout, model, style)
        } else {
            Vec::new()
        };
        if build_browser_rows_or_map {
            if model.map.active && build_map_panel {
                let canvas = compute_browser_map_canvas_rect(layout.browser_rows, sizing);
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: canvas,
                        color: blend_color(style.surface_base, style.bg_secondary, 0.24),
                    }),
                );
                push_border(
                    primitives,
                    canvas,
                    style.border_emphasis,
                    sizing.border_width,
                );
                for point in &model.map.points {
                    let center =
                        compute_browser_map_point_center(canvas, point.x_milli, point.y_milli);
                    let color = map_point_color(style, point);
                    let radius = if point.focused {
                        4.5
                    } else if point.selected {
                        3.8
                    } else {
                        2.6
                    };
                    emit_primitive(
                        primitives,
                        Primitive::Circle(FillCircle {
                            center,
                            radius,
                            color,
                        }),
                    );
                }
            } else if !model.map.active && build_browser_rows_window {
                for row in browser_rows.iter() {
                    let row_text_layout = compute_browser_row_text_layout(row.rect, sizing);
                    let row_columns = row_text_layout.columns;
                    let row_fill = if row.focused {
                        translucent_overlay_color(
                            style.bg_tertiary,
                            style.grid_strong,
                            focus_fill_emphasis,
                        )
                    } else if row.selected {
                        translucent_overlay_color(
                            style.bg_tertiary,
                            style.grid_soft,
                            style.state_selected_blend,
                        )
                    } else if row.visible_row % 2 == 0 {
                        blend_color(style.surface_base, style.bg_secondary, 0.20)
                    } else {
                        blend_color(style.surface_base, style.bg_secondary, 0.10)
                    };
                    let row_border = if row.focused {
                        blend_color(
                            style.accent_warning,
                            style.text_primary,
                            motion_wave * style.state_focus_pulse_blend,
                        )
                    } else if row.selected {
                        blend_color(
                            style.accent_mint,
                            style.text_primary,
                            motion_wave * style.state_selected_blend,
                        )
                    } else {
                        style.border
                    };
                    let row_text_color = if row.focused {
                        blend_color(
                            style.accent_warning,
                            style.text_primary,
                            motion_wave * focus_text_emphasis,
                        )
                    } else if row.selected {
                        style.accent_mint
                    } else {
                        style.text_primary
                    };
                    emit_primitive(
                        primitives,
                        Primitive::Rect(FillRect {
                            rect: row.rect,
                            color: row_fill,
                        }),
                    );
                    for separator_x in [row_columns.index.max.x, row_columns.sample.max.x] {
                        emit_primitive(
                            primitives,
                            Primitive::Rect(FillRect {
                                rect: Rect::from_min_max(
                                    Point::new(separator_x, row.rect.min.y),
                                    Point::new(
                                        (separator_x + sizing.border_width).min(row.rect.max.x),
                                        row.rect.max.y,
                                    ),
                                ),
                                color: blend_color(style.border, style.grid_soft, 0.36),
                            }),
                        );
                    }
                    push_border(
                        primitives,
                        row.rect,
                        row_border,
                        if row.focused {
                            sizing.focus_stroke_width
                        } else {
                            sizing.border_width
                        },
                    );
                    let chip_rect = row_text_layout.bucket_chip;
                    let chip_color = match row.column {
                        0 => blend_color(style.accent_warning, style.bg_secondary, 0.54),
                        2 => blend_color(style.accent_mint, style.bg_secondary, 0.54),
                        _ => blend_color(style.text_muted, style.bg_secondary, 0.54),
                    };
                    emit_primitive(
                        primitives,
                        Primitive::Rect(FillRect {
                            rect: chip_rect,
                            color: chip_color,
                        }),
                    );
                    push_border(primitives, chip_rect, style.border, sizing.border_width);
                    emit_text(
                        text_runs,
                        TextRun {
                            text: row.visible_row.to_string(),
                            position: row_text_layout.index_label.min,
                            font_size: sizing.font_meta,
                            color: style.text_muted,
                            max_width: Some(row_text_layout.index_label.width().max(12.0)),
                            align: TextAlign::Right,
                        },
                    );
                    let label_max_width = row_text_layout.sample_label.width().max(20.0);
                    emit_text(
                        text_runs,
                        TextRun {
                            text: row.label.clone(),
                            position: row_text_layout.sample_label.min,
                            font_size: sizing.font_body,
                            color: row_text_color,
                            max_width: Some(label_max_width.max(20.0)),
                            align: TextAlign::Left,
                        },
                    );
                    let bucket_label_max_width = row_text_layout.bucket_label.width().max(10.0);
                    emit_text(
                        text_runs,
                        TextRun {
                            text: row.bucket_label.clone(),
                            position: row_text_layout.bucket_label.min,
                            font_size: sizing.font_meta,
                            color: style.text_primary,
                            max_width: Some(bucket_label_max_width),
                            align: TextAlign::Center,
                        },
                    );
                }
            }
        }

        push_border(
            primitives,
            layout.top_bar,
            style.border,
            sizing.border_width,
        );
        push_border(
            primitives,
            layout.sidebar,
            style.border,
            sizing.border_width,
        );
        push_border(
            primitives,
            layout.waveform_card,
            style.border,
            sizing.border_width,
        );
        push_border(
            primitives,
            layout.browser_panel,
            style.border,
            sizing.border_width,
        );
        push_border(
            primitives,
            layout.browser_table_header,
            style.border,
            sizing.border_width,
        );
        push_border(
            primitives,
            layout.status_bar,
            style.border,
            sizing.border_width,
        );

        if build_global_static {
            let lamp_radius = sizing.lamp_radius_base
                + (((self.pulse_phase.sin() + 1.0) * 0.5) * sizing.lamp_radius_amp);
            let lamp_color = if self.transport_running {
                style.accent_mint
            } else {
                style.accent_copper
            };
            emit_primitive(
                primitives,
                Primitive::Circle(FillCircle {
                    center: Point::new(
                        layout.top_bar.max.x - (sizing.text_inset_x + 14.0),
                        layout.top_bar_title_row.min.y + (layout.top_bar_title_row.height() * 0.5),
                    ),
                    radius: lamp_radius,
                    color: lamp_color,
                }),
            );

            let _top_title_rect = compute_top_bar_title_text_rect(
                layout.top_bar_title_cluster,
                layout.top_bar_title_row,
                sizing,
            );
            let top_controls = top_bar_controls_layout(layout, sizing);
            if top_controls.active {
                let top_controls_text = compute_top_bar_controls_text_layout(
                    top_controls.options_label,
                    top_controls.volume_value,
                    top_controls.volume_label,
                    sizing,
                );
                let divider_y = layout.top_bar_controls_row.min.y;
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: Rect::from_min_max(
                            Point::new(layout.top_bar.min.x, divider_y),
                            Point::new(
                                layout.top_bar.max.x,
                                (divider_y + sizing.border_width).min(layout.top_bar.max.y),
                            ),
                        ),
                        color: style.border,
                    }),
                );
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: top_controls.volume_meter,
                        color: style.surface_overlay,
                    }),
                );
                push_border(
                    primitives,
                    top_controls.volume_meter,
                    style.border_emphasis,
                    sizing.border_width,
                );
                let volume_level = model.volume.clamp(0.0, 1.0);
                let fill_width = (top_controls.volume_meter.width() * volume_level)
                    .clamp(1.0, top_controls.volume_meter.width());
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: Rect::from_min_max(
                            top_controls.volume_meter.min,
                            Point::new(
                                top_controls.volume_meter.min.x + fill_width,
                                top_controls.volume_meter.max.y,
                            ),
                        ),
                        color: blend_color(style.accent_mint, style.text_primary, 0.28),
                    }),
                );
                emit_text(
                    text_runs,
                    TextRun {
                        text: String::from("Options"),
                        position: top_controls_text.options_label.min,
                        font_size: sizing.font_meta,
                        color: style.text_primary,
                        max_width: Some(top_controls_text.options_label.width().max(24.0)),
                        align: TextAlign::Left,
                    },
                );
                emit_text(
                    text_runs,
                    TextRun {
                        text: format!("{volume_level:.2}"),
                        position: top_controls_text.volume_value.min,
                        font_size: sizing.font_meta,
                        color: style.text_muted,
                        max_width: Some(top_controls_text.volume_value.width().max(20.0)),
                        align: TextAlign::Right,
                    },
                );
                emit_text(
                    text_runs,
                    TextRun {
                        text: String::from("Vol"),
                        position: top_controls_text.volume_label.min,
                        font_size: sizing.font_meta,
                        color: style.text_muted,
                        max_width: Some(top_controls_text.volume_label.width().max(18.0)),
                        align: TextAlign::Left,
                    },
                );
            }
            let update_buttons = update_action_buttons(layout, style, model);
            let update_status_text = update_status_text(model);
            let update_hint_text = update_hint_text(model);
            let update_notes_text = update_notes_text(model);
            let update_controls_text = if update_notes_text.is_empty() {
                update_hint_text
            } else if update_hint_text.is_empty() {
                update_notes_text
            } else {
                format!("{update_hint_text} | {update_notes_text}")
            };
            let update_button_rects: Vec<Rect> =
                update_buttons.iter().map(|button| button.rect).collect();
            let update_text_layout = compute_top_bar_update_text_layout(
                layout.top_bar_action_cluster,
                layout.top_bar_title_row,
                layout.top_bar_controls_row,
                sizing,
                &update_button_rects,
            );
            let update_status_width = update_text_layout.status_line.width().max(20.0);
            emit_text(
                text_runs,
                TextRun {
                    text: truncate_to_width(
                        &update_status_text,
                        update_status_width,
                        sizing.font_meta,
                    ),
                    position: update_text_layout.status_line.min,
                    font_size: sizing.font_meta,
                    color: style.text_muted,
                    max_width: Some(update_status_width),
                    align: TextAlign::Left,
                },
            );
            if !update_controls_text.is_empty() && update_text_layout.controls_line.width() > 0.0 {
                let controls_width = update_text_layout.controls_line.width().max(20.0);
                emit_text(
                    text_runs,
                    TextRun {
                        text: truncate_to_width(
                            &update_controls_text,
                            controls_width,
                            sizing.font_meta,
                        ),
                        position: update_text_layout.controls_line.min,
                        font_size: sizing.font_meta,
                        color: style.text_muted,
                        max_width: Some(controls_width),
                        align: TextAlign::Left,
                    },
                );
            }
            for button in &update_buttons {
                let label_rect = compute_action_button_text_rect(button.rect, sizing);
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: button.rect,
                        color: if button.enabled {
                            style.surface_overlay
                        } else {
                            style.control_disabled_fill
                        },
                    }),
                );
                push_border(
                    primitives,
                    button.rect,
                    if button.enabled {
                        blend_color(
                            style.border_emphasis,
                            style.text_primary,
                            style.state_hover_soft,
                        )
                    } else {
                        style.border
                    },
                    sizing.border_width,
                );
                emit_text(
                    text_runs,
                    TextRun {
                        text: button.label.to_string(),
                        position: label_rect.min,
                        font_size: sizing.font_meta,
                        color: if button.enabled {
                            button.text_color
                        } else {
                            style.text_muted
                        },
                        max_width: Some(label_rect.width().max(12.0)),
                        align: TextAlign::Center,
                    },
                );
            }
        }
        if build_browser_frame {
            for button in &browser_buttons {
                let label_rect = compute_action_button_text_rect(button.rect, sizing);
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: button.rect,
                        color: if button.enabled {
                            style.surface_overlay
                        } else {
                            style.control_disabled_fill
                        },
                    }),
                );
                push_border(
                    primitives,
                    button.rect,
                    if button.enabled {
                        blend_color(
                            style.border_emphasis,
                            style.text_primary,
                            style.state_hover_soft,
                        )
                    } else {
                        style.border
                    },
                    sizing.border_width,
                );
                emit_text(
                    text_runs,
                    TextRun {
                        text: button.label.to_string(),
                        position: label_rect.min,
                        font_size: sizing.font_meta,
                        color: if button.enabled {
                            button.text_color
                        } else {
                            style.text_muted
                        },
                        max_width: Some(label_rect.width().max(12.0)),
                        align: TextAlign::Center,
                    },
                );
            }
        }
        if build_global_static {
            let sources_header = if model.sources.header.is_empty() {
                model.sources_label.as_str()
            } else {
                model.sources.header.as_str()
            };
            let sidebar_sections = sidebar_sections(layout, style, model);
            let sidebar_header_text =
                compute_sidebar_header_text_layout(layout.sidebar_header, sizing);
            let sidebar_header_title_width = sidebar_header_text.title_row.width().max(72.0);
            let sidebar_header_query_width = sidebar_header_text.query_row.width().max(72.0);
            emit_text(
                text_runs,
                TextRun {
                    text: truncate_to_width(
                        sources_header,
                        sidebar_header_title_width,
                        sizing.font_header,
                    ),
                    position: sidebar_header_text.title_row.min,
                    font_size: sizing.font_header,
                    color: style.text_primary,
                    max_width: Some(sidebar_header_title_width),
                    align: TextAlign::Left,
                },
            );
            emit_text(
                text_runs,
                TextRun {
                    text: format!(
                        "search: {}",
                        if model.sources.search_query.is_empty() {
                            "—"
                        } else {
                            model.sources.search_query.as_str()
                        }
                    ),
                    position: sidebar_header_text.query_row.min,
                    font_size: sizing.font_meta,
                    color: style.text_muted,
                    max_width: Some(sidebar_header_query_width),
                    align: TextAlign::Left,
                },
            );
            let rendered_sources = source_row_rects.len();
            for (row_index, row_rect) in source_row_rects.iter().enumerate() {
                let row_rect = *row_rect;
                let row = &model.sources.rows[row_index];
                let row_selected = row.selected
                    || model
                        .sources
                        .selected_row
                        .is_some_and(|selected| selected == row_index);
                let row_fill = if row_selected {
                    translucent_overlay_color(
                        style.bg_tertiary,
                        style.grid_soft,
                        style.state_selected_blend,
                    )
                } else {
                    style.surface_base
                };
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: row_rect,
                        color: row_fill,
                    }),
                );
                push_border(
                    primitives,
                    row_rect,
                    if row_selected {
                        blend_color(
                            style.accent_mint,
                            style.text_primary,
                            motion_wave * style.state_selected_blend,
                        )
                    } else if row.missing {
                        style.accent_warning
                    } else {
                        style.border
                    },
                    sizing.border_width,
                );
                let source_label_rect = compute_sidebar_source_row_text_rect(row_rect, sizing);
                emit_text(
                    text_runs,
                    TextRun {
                        text: truncate_to_width(
                            &row.label,
                            source_label_rect.width().max(24.0),
                            sizing.font_body,
                        ),
                        position: source_label_rect.min,
                        font_size: sizing.font_body,
                        color: if row_selected {
                            style.accent_mint
                        } else {
                            style.text_primary
                        },
                        max_width: Some(source_label_rect.width().max(24.0)),
                        align: TextAlign::Left,
                    },
                );
            }
            let rendered_folders = folder_row_rects.len();
            if rendered_folders > 0 {
                if let Some(divider_rect) = compute_source_section_divider_rect(
                    sidebar_sections.source_rows,
                    sidebar_sections.folder_header,
                    sizing,
                ) {
                    emit_primitive(
                        primitives,
                        Primitive::Rect(FillRect {
                            rect: divider_rect,
                            color: style.source_section_divider,
                        }),
                    );
                }
                let folder_header_layout = compute_sidebar_folder_header_layout(
                    sidebar_sections.folder_header,
                    sizing,
                    model.sources.folder_recovery.in_progress,
                    model.sources.folder_recovery.entry_count,
                );
                if let Some(badge) = folder_header_layout.badge.as_ref() {
                    emit_primitive(
                        primitives,
                        Primitive::Rect(FillRect {
                            rect: badge.rect,
                            color: if badge.active {
                                style.source_recovery_badge_active
                            } else {
                                style.source_recovery_badge_idle
                            },
                        }),
                    );
                    push_border(
                        primitives,
                        badge.rect,
                        blend_color(
                            style.border_emphasis,
                            style.text_primary,
                            style.state_hover_soft,
                        ),
                        sizing.border_width,
                    );
                    let badge_text_rect =
                        compute_sidebar_recovery_badge_text_rect(badge.rect, sizing);
                    emit_text(
                        text_runs,
                        TextRun {
                            text: badge.label.clone(),
                            position: badge_text_rect.min,
                            font_size: sizing.font_meta,
                            color: style.text_primary,
                            max_width: Some(badge_text_rect.width().max(18.0)),
                            align: TextAlign::Center,
                        },
                    );
                }
                if folder_header_layout.title_row.width() > 8.0 {
                    emit_text(
                        text_runs,
                        TextRun {
                            text: format!("Folders ({})", model.sources.folder_rows.len()),
                            position: Point::new(
                                folder_header_layout.title_row.min.x,
                                folder_header_layout.title_row.min.y,
                            ),
                            font_size: sizing.font_header,
                            color: style.text_primary,
                            max_width: Some(folder_header_layout.title_row.width()),
                            align: TextAlign::Left,
                        },
                    );
                    if let Some(metadata_row) = folder_header_layout
                        .metadata_row
                        .filter(|row| row.width() > 24.0)
                    {
                        emit_text(
                            text_runs,
                            TextRun {
                                text: format!(
                                    "query: {}",
                                    if model.sources.folder_search_query.is_empty() {
                                        "—"
                                    } else {
                                        model.sources.folder_search_query.as_str()
                                    }
                                ),
                                position: metadata_row.min,
                                font_size: sizing.font_meta,
                                color: style.text_muted,
                                max_width: Some(metadata_row.width()),
                                align: TextAlign::Left,
                            },
                        );
                    }
                }
                for (row_index, row_rect) in folder_row_rects.iter().enumerate() {
                    let row_rect = *row_rect;
                    let row = &model.sources.folder_rows[row_index];
                    let row_selected = row.selected || row.focused;
                    let row_fill = if row.focused {
                        translucent_overlay_color(
                            style.bg_tertiary,
                            style.grid_strong,
                            (style.state_hover_soft + (motion_wave * style.motion_focus_wave_amp))
                                .clamp(0.0, 1.0),
                        )
                    } else if row_selected {
                        translucent_overlay_color(
                            style.bg_tertiary,
                            style.grid_soft,
                            style.state_selected_blend,
                        )
                    } else {
                        style.surface_base
                    };
                    emit_primitive(
                        primitives,
                        Primitive::Rect(FillRect {
                            rect: row_rect,
                            color: row_fill,
                        }),
                    );
                    push_border(
                        primitives,
                        row_rect,
                        if row.focused {
                            blend_color(
                                style.accent_warning,
                                style.text_primary,
                                motion_wave * style.state_focus_pulse_blend,
                            )
                        } else if row.selected {
                            blend_color(
                                style.accent_mint,
                                style.text_primary,
                                motion_wave * style.state_selected_blend,
                            )
                        } else {
                            style.border
                        },
                        if row.focused {
                            sizing.focus_stroke_width
                        } else {
                            sizing.border_width
                        },
                    );
                    let glyph = if row.is_root {
                        "•"
                    } else if row.has_children {
                        if row.expanded { "▼" } else { "▶" }
                    } else {
                        "·"
                    };
                    let depth_indent = (row.depth as f32 * sizing.folder_indent_step)
                        .min((row_rect.width() * 0.45).max(0.0));
                    let label_text = format!("{glyph} {}", row.label);
                    let folder_text_rect =
                        compute_sidebar_folder_row_text_rect(row_rect, sizing, depth_indent);
                    let folder_text_width = folder_text_rect.width().max(24.0);
                    emit_text(
                        text_runs,
                        TextRun {
                            text: truncate_to_width(
                                &label_text,
                                folder_text_width,
                                sizing.font_body,
                            ),
                            position: folder_text_rect.min,
                            font_size: sizing.font_body,
                            color: if row.focused {
                                style.accent_warning
                            } else if row.selected {
                                style.accent_mint
                            } else {
                                style.text_primary
                            },
                            max_width: Some(folder_text_width),
                            align: TextAlign::Left,
                        },
                    );
                }
            }
            if let Some((menu_panel, menu_buttons)) =
                source_context_menu_spec(layout, style, model, self.source_context_menu)
            {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: menu_panel,
                        color: style.surface_overlay,
                    }),
                );
                push_border(
                    primitives,
                    menu_panel,
                    style.border_emphasis,
                    sizing.border_width,
                );
                for button in menu_buttons {
                    let label_rect = compute_action_button_text_rect(button.rect, sizing);
                    emit_primitive(
                        primitives,
                        Primitive::Rect(FillRect {
                            rect: button.rect,
                            color: if button.enabled {
                                blend_color(style.surface_base, style.bg_secondary, 0.36)
                            } else {
                                style.control_disabled_fill
                            },
                        }),
                    );
                    push_border(
                        primitives,
                        button.rect,
                        if button.enabled {
                            style.border
                        } else {
                            style.grid_soft
                        },
                        sizing.border_width,
                    );
                    emit_text(
                        text_runs,
                        TextRun {
                            text: button.label.to_string(),
                            position: label_rect.min,
                            font_size: sizing.font_meta,
                            color: if button.enabled {
                                button.text_color
                            } else {
                                style.text_muted
                            },
                            max_width: Some(label_rect.width().max(16.0)),
                            align: TextAlign::Center,
                        },
                    );
                }
            }
            for button in source_action_buttons(layout, style, model) {
                let label_rect = compute_action_button_text_rect(button.rect, sizing);
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: button.rect,
                        color: if button.enabled {
                            style.surface_overlay
                        } else {
                            style.control_disabled_fill
                        },
                    }),
                );
                push_border(
                    primitives,
                    button.rect,
                    if button.enabled {
                        blend_color(
                            style.border_emphasis,
                            style.text_primary,
                            style.state_hover_soft,
                        )
                    } else {
                        style.border
                    },
                    sizing.border_width,
                );
                emit_text(
                    text_runs,
                    TextRun {
                        text: button.label.to_string(),
                        position: label_rect.min,
                        font_size: sizing.font_meta,
                        color: if button.enabled {
                            button.text_color
                        } else {
                            style.text_muted
                        },
                        max_width: Some(label_rect.width().max(12.0)),
                        align: TextAlign::Center,
                    },
                );
            }
            let sidebar_footer_text =
                compute_sidebar_footer_text_layout(layout.sidebar_footer, sizing);
            let sidebar_footer_primary_width = sidebar_footer_text.primary_row.width().max(56.0);
            let sidebar_footer_secondary_width =
                sidebar_footer_text.secondary_row.width().max(56.0);
            if model.sources.rows.len() > rendered_sources {
                emit_text(
                    text_runs,
                    TextRun {
                        text: format!("+{} more…", model.sources.rows.len() - rendered_sources),
                        position: sidebar_footer_text.primary_row.min,
                        font_size: sizing.font_meta,
                        color: style.text_muted,
                        max_width: Some(sidebar_footer_primary_width),
                        align: TextAlign::Left,
                    },
                );
            }
            if model.sources.folder_rows.len() > rendered_folders {
                emit_text(
                    text_runs,
                    TextRun {
                        text: format!(
                            "folders: +{} more…",
                            model.sources.folder_rows.len() - rendered_folders
                        ),
                        position: sidebar_footer_text.secondary_row.min,
                        font_size: sizing.font_meta,
                        color: style.text_muted,
                        max_width: Some(sidebar_footer_secondary_width),
                        align: TextAlign::Left,
                    },
                );
            } else if model.sources.folder_recovery.entry_count > 0 {
                emit_text(
                    text_runs,
                    TextRun {
                        text: format!(
                            "recovery entries: {}",
                            model.sources.folder_recovery.entry_count
                        ),
                        position: sidebar_footer_text.secondary_row.min,
                        font_size: sizing.font_meta,
                        color: style.text_muted,
                        max_width: Some(sidebar_footer_secondary_width),
                        align: TextAlign::Left,
                    },
                );
            }
        }
        // Waveform summary text is produced during overlay rendering so it can
        // update while transport advances without invalidating the static scene.
        if build_browser_frame {
            let tabs = compute_browser_tabs_rects(layout.browser_tabs, sizing);
            let map_active = model.map.active;
            let list_active = !map_active;
            let (
                samples_fill,
                map_fill,
                samples_border,
                map_border,
                samples_text_color,
                map_text_color,
            ) = if list_active {
                (
                    blend_color(
                        style.surface_overlay,
                        style.bg_tertiary,
                        style.state_selected_blend + (motion_wave * 0.1),
                    ),
                    style.surface_base,
                    blend_color(style.accent_mint, style.text_primary, 0.42),
                    style.border,
                    blend_color(
                        style.accent_mint,
                        style.text_primary,
                        motion_wave * style.state_selected_blend,
                    ),
                    style.text_muted,
                )
            } else {
                (
                    style.surface_base,
                    blend_color(
                        style.surface_overlay,
                        style.bg_tertiary,
                        style.state_selected_blend + (motion_wave * 0.1),
                    ),
                    style.border,
                    blend_color(style.accent_mint, style.text_primary, 0.42),
                    style.text_muted,
                    blend_color(
                        style.accent_mint,
                        style.text_primary,
                        motion_wave * style.state_selected_blend,
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
            push_border(
                primitives,
                tabs.samples,
                samples_border,
                sizing.border_width,
            );
            push_border(primitives, tabs.map, map_border, sizing.border_width);
            let tabs_text_layout = compute_browser_tabs_text_layout(tabs.samples, tabs.map, sizing);
            let samples_text = format!(
                "{} ({})",
                model.browser_chrome.samples_tab_label,
                model.columns.get(1).map(|c| c.item_count).unwrap_or(0)
            );
            emit_text(
                text_runs,
                TextRun {
                    text: truncate_to_width(
                        &samples_text,
                        tabs_text_layout.samples_label.width().max(40.0),
                        sizing.font_header,
                    ),
                    position: tabs_text_layout.samples_label.min,
                    font_size: sizing.font_header,
                    color: samples_text_color,
                    max_width: Some(tabs_text_layout.samples_label.width().max(40.0)),
                    align: TextAlign::Left,
                },
            );
            let map_text = model.browser_chrome.map_tab_label.as_str();
            emit_text(
                text_runs,
                TextRun {
                    text: String::from(map_text),
                    position: tabs_text_layout.map_label.min,
                    font_size: sizing.font_header,
                    color: map_text_color,
                    max_width: Some(tabs_text_layout.map_label.width().max(40.0)),
                    align: TextAlign::Left,
                },
            );
            let toolbar = browser_toolbar_layout(layout, style, &browser_buttons);
            if toolbar.search_field.width() > 1.0 {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: toolbar.search_field,
                        color: style.surface_overlay,
                    }),
                );
                push_border(
                    primitives,
                    toolbar.search_field,
                    blend_color(style.border_emphasis, style.text_primary, 0.35),
                    sizing.border_width,
                );
            }
            if toolbar.activity_chip.width() > 1.0 {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: toolbar.activity_chip,
                        color: if model.browser.busy {
                            blend_color(style.accent_warning, style.bg_secondary, 0.45)
                        } else {
                            blend_color(style.accent_mint, style.bg_secondary, 0.40)
                        },
                    }),
                );
                push_border(
                    primitives,
                    toolbar.activity_chip,
                    style.border,
                    sizing.border_width,
                );
            }
            if toolbar.sort_chip.width() > 1.0 {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: toolbar.sort_chip,
                        color: style.surface_overlay,
                    }),
                );
                push_border(
                    primitives,
                    toolbar.sort_chip,
                    style.border,
                    sizing.border_width,
                );
            }
            for chip in &browser_column_chips {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: chip.rect,
                        color: if chip.selected {
                            match chip.column {
                                0 => blend_color(style.accent_warning, style.bg_secondary, 0.50),
                                2 => blend_color(style.accent_mint, style.bg_secondary, 0.50),
                                _ => blend_color(style.text_primary, style.bg_secondary, 0.42),
                            }
                        } else {
                            match chip.column {
                                0 => blend_color(style.accent_warning, style.bg_secondary, 0.34),
                                2 => blend_color(style.accent_mint, style.bg_secondary, 0.34),
                                _ => blend_color(style.text_muted, style.bg_secondary, 0.28),
                            }
                        },
                    }),
                );
                push_border(
                    primitives,
                    chip.rect,
                    if chip.selected {
                        blend_color(style.border_emphasis, style.text_primary, 0.55)
                    } else {
                        style.border
                    },
                    sizing.border_width,
                );
                let label_rect = compute_action_button_text_rect(chip.rect, sizing);
                emit_text(
                    text_runs,
                    TextRun {
                        text: format!("{} ({})", chip.label, chip.item_count),
                        position: label_rect.min,
                        font_size: sizing.font_meta,
                        color: if chip.selected {
                            style.text_primary
                        } else {
                            style.text_muted
                        },
                        max_width: Some(label_rect.width().max(16.0)),
                        align: TextAlign::Center,
                    },
                );
            }
            let toolbar_text_layout = compute_browser_toolbar_text_layout(
                toolbar.search_field,
                toolbar.activity_chip,
                toolbar.sort_chip,
                sizing,
            );
            let search_text = if model.browser.search_query.is_empty() {
                model.browser_chrome.search_placeholder.clone()
            } else {
                format!(
                    "{}: {}",
                    model.browser_chrome.search_prefix_label, model.browser.search_query
                )
            };
            if toolbar.search_field.width() > 1.0 {
                emit_text(
                    text_runs,
                    TextRun {
                        text: truncate_to_width(
                            &search_text,
                            toolbar_text_layout.search_label.width().max(24.0),
                            sizing.font_meta,
                        ),
                        position: toolbar_text_layout.search_label.min,
                        font_size: sizing.font_meta,
                        color: if model.browser.search_query.is_empty() {
                            style.text_muted
                        } else {
                            style.text_primary
                        },
                        max_width: Some(toolbar_text_layout.search_label.width().max(24.0)),
                        align: TextAlign::Left,
                    },
                );
            }
            if toolbar.activity_chip.width() > 1.0 {
                emit_text(
                    text_runs,
                    TextRun {
                        text: if model.browser.busy {
                            model.browser_chrome.activity_busy_label.clone()
                        } else {
                            model.browser_chrome.activity_ready_label.clone()
                        },
                        position: toolbar_text_layout.activity_label.min,
                        font_size: sizing.font_meta,
                        color: style.text_primary,
                        max_width: Some(toolbar_text_layout.activity_label.width().max(20.0)),
                        align: TextAlign::Center,
                    },
                );
            }
            if toolbar.sort_chip.width() > 1.0 {
                let sort_label = if model.browser_chrome.sort_order_label.is_empty() {
                    model.browser.sort_label.as_deref().unwrap_or("List order")
                } else {
                    model.browser_chrome.sort_order_label.as_str()
                };
                let sort_text = if model.browser_chrome.sort_prefix_label.is_empty() {
                    String::from(sort_label)
                } else {
                    format!("{}: {}", model.browser_chrome.sort_prefix_label, sort_label)
                };
                emit_text(
                    text_runs,
                    TextRun {
                        text: sort_text,
                        position: toolbar_text_layout.sort_label.min,
                        font_size: sizing.font_meta,
                        color: style.text_muted,
                        max_width: Some(toolbar_text_layout.sort_label.width().max(20.0)),
                        align: TextAlign::Center,
                    },
                );
            }
        }
        if model.map.active && build_map_panel {
            let mode_label = match model.map.render_mode {
                crate::app::MapRenderModeModel::Heatmap => "heatmap",
                crate::app::MapRenderModeModel::Points => "points",
            };
            let legend_text = if model.map.legend_label.is_empty() {
                format!(
                    "{}: {mode_label}",
                    model.browser_chrome.similarity_toggle_label
                )
            } else {
                model.map.legend_label.clone()
            };
            let header_left_text =
                format!("{} | {}", model.browser_chrome.map_tab_label, legend_text);
            let selection_or_error = if let Some(error) = model.map.error.as_deref() {
                (error.to_string(), style.accent_warning)
            } else if !model.map.selection_label.is_empty() {
                (model.map.selection_label.clone(), style.text_muted)
            } else if !model.map.hover_label.is_empty() {
                (model.map.hover_label.clone(), style.text_muted)
            } else {
                (String::from("Selection: —"), style.text_muted)
            };
            let map_header_text_layout =
                compute_browser_map_header_text_layout(layout.browser_table_header, sizing);
            let left_max_width = map_header_text_layout.left_label.width().max(24.0);
            let right_max_width = map_header_text_layout.right_label.width().max(36.0);
            emit_text(
                text_runs,
                TextRun {
                    text: truncate_to_width(&header_left_text, left_max_width, sizing.font_meta),
                    position: map_header_text_layout.left_label.min,
                    font_size: sizing.font_meta,
                    color: style.text_primary,
                    max_width: Some(left_max_width),
                    align: TextAlign::Left,
                },
            );
            emit_text(
                text_runs,
                TextRun {
                    text: truncate_to_width(
                        &selection_or_error.0,
                        right_max_width,
                        sizing.font_meta,
                    ),
                    position: map_header_text_layout.right_label.min,
                    font_size: sizing.font_meta,
                    color: selection_or_error.1,
                    max_width: Some(right_max_width),
                    align: TextAlign::Right,
                },
            );
        } else if build_browser_frame {
            let header_text_layout =
                compute_browser_header_text_layout(layout.browser_table_header, sizing);
            let header = header_text_layout.columns;
            for separator_x in [header.index.max.x, header.sample.max.x] {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: Rect::from_min_max(
                            Point::new(separator_x, layout.browser_table_header.min.y),
                            Point::new(
                                (separator_x + sizing.border_width)
                                    .min(layout.browser_table_header.max.x),
                                layout.browser_table_header.max.y,
                            ),
                        ),
                        color: style.border,
                    }),
                );
            }
            emit_text(
                text_runs,
                TextRun {
                    text: String::from("#"),
                    position: header_text_layout.index_label.min,
                    font_size: sizing.font_meta,
                    color: style.text_muted,
                    max_width: Some(header_text_layout.index_label.width().max(12.0)),
                    align: TextAlign::Right,
                },
            );
            emit_text(
                text_runs,
                TextRun {
                    text: String::from("Sample"),
                    position: header_text_layout.sample_label.min,
                    font_size: sizing.font_meta,
                    color: style.text_primary,
                    max_width: Some(header_text_layout.sample_label.width().max(24.0)),
                    align: TextAlign::Left,
                },
            );
            emit_text(
                text_runs,
                TextRun {
                    text: String::from("Bucket"),
                    position: header_text_layout.bucket_label.min,
                    font_size: sizing.font_meta,
                    color: style.text_primary,
                    max_width: Some(header_text_layout.bucket_label.width().max(20.0)),
                    align: TextAlign::Center,
                },
            );
        }
        if build_browser_frame {
            let footer_text = if model.map.active {
                let mut parts = Vec::new();
                if !model.map.summary.is_empty() {
                    parts.push(model.map.summary.clone());
                }
                if !model.map.cluster_label.is_empty() {
                    parts.push(model.map.cluster_label.clone());
                }
                if !model.map.hover_label.is_empty() {
                    parts.push(model.map.hover_label.clone());
                }
                if !model.map.viewport_label.is_empty() {
                    parts.push(model.map.viewport_label.clone());
                }
                if parts.is_empty() {
                    model.map.summary.clone()
                } else {
                    parts.join(" | ")
                }
            } else {
                format!(
                    "{} | {} selected{}",
                    model.browser_chrome.item_count_label,
                    model.browser.selected_path_count,
                    if model.browser.busy {
                        " | filtering…"
                    } else {
                        ""
                    }
                )
            };
            let browser_footer_text =
                compute_browser_footer_text_rect(layout.browser_footer, sizing);
            emit_text(
                text_runs,
                TextRun {
                    text: truncate_to_width(
                        &footer_text,
                        browser_footer_text.width().max(36.0),
                        sizing.font_meta,
                    ),
                    position: browser_footer_text.min,
                    font_size: sizing.font_meta,
                    color: style.text_muted,
                    max_width: Some(browser_footer_text.width().max(36.0)),
                    align: TextAlign::Left,
                },
            );
        }

        if build_status_bar {
            let status_text = if model.status_text.is_empty() {
                if self.transport_running {
                    format!(
                        "Transport: running | Selected column: {}",
                        self.selected_column + 1
                    )
                } else {
                    format!(
                        "Transport: stopped | Selected column: {}",
                        self.selected_column + 1
                    )
                }
            } else {
                model.status_text.clone()
            };
            let browser_summary = format!(
                "rows: {} | selected: {} | anchor: {} | search: {}{}",
                model.browser.visible_count,
                model.browser.selected_path_count,
                model
                    .browser
                    .anchor_visible_row
                    .map(|row| row.to_string())
                    .unwrap_or_else(|| String::from("—")),
                if model.browser.search_query.is_empty() {
                    "—"
                } else {
                    model.browser.search_query.as_str()
                },
                if model.browser.busy {
                    " | filtering…"
                } else {
                    ""
                }
            );
            let status_left = if model.status.left.is_empty() {
                status_text
            } else {
                model.status.left.clone()
            };
            let status_center = if model.status.center.is_empty() {
                browser_summary
            } else {
                model.status.center.clone()
            };
            let status_right = if model.status.right.is_empty() {
                format!("col: {}/3", self.selected_column + 1)
            } else {
                model.status.right.clone()
            };
            let status_left_text_rect = compute_status_text_line_rect(
                layout.status_left_segment,
                sizing,
                sizing.font_status,
            );
            let status_center_text_rect = compute_status_text_line_rect(
                layout.status_center_segment,
                sizing,
                sizing.font_status,
            );
            let status_right_text_rect = compute_status_text_line_rect(
                layout.status_right_segment,
                sizing,
                sizing.font_status,
            );
            emit_text(
                text_runs,
                TextRun {
                    text: truncate_to_width(
                        &status_left,
                        status_left_text_rect.width().max(36.0),
                        sizing.font_status,
                    ),
                    position: status_left_text_rect.min,
                    font_size: sizing.font_status,
                    color: style.text_muted,
                    max_width: Some(status_left_text_rect.width().max(36.0)),
                    align: TextAlign::Left,
                },
            );
            emit_text(
                text_runs,
                TextRun {
                    text: truncate_to_width(
                        &status_center,
                        status_center_text_rect.width().max(36.0),
                        sizing.font_status,
                    ),
                    position: status_center_text_rect.min,
                    font_size: sizing.font_status,
                    color: style.text_primary,
                    max_width: Some(status_center_text_rect.width().max(36.0)),
                    align: TextAlign::Center,
                },
            );
            emit_text(
                text_runs,
                TextRun {
                    text: truncate_to_width(
                        &status_right,
                        status_right_text_rect.width().max(36.0),
                        sizing.font_status,
                    ),
                    position: status_right_text_rect.min,
                    font_size: sizing.font_status,
                    color: style.text_muted,
                    max_width: Some(status_right_text_rect.width().max(36.0)),
                    align: TextAlign::Right,
                },
            );
        }

        if include_overlays {
            render_progress_overlay(primitives, text_runs, layout, style, model);
            render_confirm_prompt(primitives, text_runs, layout, style, model);
            render_drag_overlay(primitives, text_runs, layout, style, model);
        }
    }

    /// Build only state-driven overlays into reusable buffers.
    pub(crate) fn build_state_overlay_into(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
        frame: &mut NativeViewFrame,
    ) {
        let sizing = style.sizing;
        frame.primitives.clear();
        frame.text_runs.clear();
        let primitives = &mut frame.primitives;
        let text_runs = &mut frame.text_runs;

        if self.hovered == Some(ShellNodeKind::TopBar) {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: layout.top_bar,
                    color: tinted_overlay_color(style.bg_tertiary, style.sizing.hover_fill_alpha),
                }),
            );
        }

        if self.hovered == Some(ShellNodeKind::Sidebar) {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: layout.sidebar,
                    color: tinted_overlay_color(style.bg_tertiary, style.sizing.hover_fill_alpha),
                }),
            );
        }

        if self.hovered == Some(ShellNodeKind::WaveformCard) {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: layout.waveform_card,
                    color: tinted_overlay_color(style.bg_tertiary, style.sizing.hover_fill_alpha),
                }),
            );
        }
        if let Some(hovered_visible_row) = self.hovered_browser_visible_row {
            let browser_rows = self.cached_browser_rows(layout, style, model);
            if let Some(row) = browser_rows
                .iter()
                .find(|row| row.visible_row == hovered_visible_row)
            {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: row.rect,
                        color: subtle_item_hover_fill(style),
                    }),
                );
            }
        }

        if self.has_focus_emphasis {
            {
                let source_row_rects = self.cached_source_row_rects(layout, style, model);
                for (row_index, row_rect) in source_row_rects.iter().enumerate() {
                    let Some(row) = model.sources.rows.get(row_index) else {
                        continue;
                    };
                    let row_selected = row.selected
                        || model
                            .sources
                            .selected_row
                            .is_some_and(|selected| selected == row_index);
                    if !row_selected {
                        continue;
                    }
                    push_border(
                        primitives,
                        *row_rect,
                        blend_color(
                            style.accent_mint,
                            style.text_primary,
                            style.state_selected_blend,
                        ),
                        sizing.border_width,
                    );
                }
            }

            {
                let folder_row_rects = self.cached_folder_row_rects(layout, style, model);
                for (row_index, row_rect) in folder_row_rects.iter().enumerate() {
                    let Some(row) = model.sources.folder_rows.get(row_index) else {
                        continue;
                    };
                    if !(row.selected || row.focused) {
                        continue;
                    }
                    if row.focused {
                        emit_primitive(
                            primitives,
                            Primitive::Rect(FillRect {
                                rect: *row_rect,
                                color: translucent_overlay_color(
                                    style.bg_tertiary,
                                    style.grid_strong,
                                    style.state_focus_pulse_blend,
                                ),
                            }),
                        );
                    }
                    if row.focused || row.selected {
                        push_border(
                            primitives,
                            *row_rect,
                            if row.focused {
                                blend_color(
                                    style.accent_warning,
                                    style.text_primary,
                                    style.state_focus_pulse_blend,
                                )
                            } else {
                                blend_color(
                                    style.accent_mint,
                                    style.text_primary,
                                    style.state_selected_blend,
                                )
                            },
                            if row.focused {
                                sizing.focus_stroke_width
                            } else {
                                sizing.border_width
                            },
                        );
                    }
                    if row.focused {
                        let glyph = if row.is_root {
                            "•"
                        } else if row.has_children {
                            if row.expanded { "▼" } else { "▶" }
                        } else {
                            "·"
                        };
                        let depth_indent = (row.depth as f32 * sizing.folder_indent_step)
                            .min((row_rect.width() * 0.45).max(0.0));
                        let row_text_rect =
                            compute_sidebar_folder_row_text_rect(*row_rect, sizing, depth_indent);
                        let row_text_width = row_text_rect.width().max(24.0);
                        let row_label = format!("{glyph} {}", row.label);
                        emit_text(
                            text_runs,
                            TextRun {
                                text: truncate_to_width(
                                    &row_label,
                                    row_text_width,
                                    sizing.font_body,
                                ),
                                position: row_text_rect.min,
                                font_size: sizing.font_body,
                                color: blend_color(
                                    style.accent_warning,
                                    style.text_primary,
                                    style.state_focus_pulse_blend,
                                ),
                                max_width: Some(row_text_width),
                                align: TextAlign::Left,
                            },
                        );
                    }
                }
            }

            {
                let browser_rows = self.cached_browser_rows(layout, style, model);
                for row in browser_rows.iter() {
                    if !(row.selected || row.focused) {
                        continue;
                    }
                    let row_text_layout = compute_browser_row_text_layout(row.rect, sizing);
                    if row.focused {
                        emit_primitive(
                            primitives,
                            Primitive::Rect(FillRect {
                                rect: row.rect,
                                color: translucent_overlay_color(
                                    style.bg_tertiary,
                                    style.grid_strong,
                                    style.state_focus_pulse_blend,
                                ),
                            }),
                        );
                    }
                    push_border(
                        primitives,
                        row.rect,
                        if row.focused {
                            blend_color(
                                style.accent_warning,
                                style.text_primary,
                                style.state_focus_pulse_blend,
                            )
                        } else {
                            blend_color(
                                style.accent_mint,
                                style.text_primary,
                                style.state_selected_blend,
                            )
                        },
                        if row.focused {
                            sizing.focus_stroke_width
                        } else {
                            sizing.border_width
                        },
                    );
                    if row.focused {
                        emit_text(
                            text_runs,
                            TextRun {
                                text: row.visible_row.to_string(),
                                position: row_text_layout.index_label.min,
                                font_size: sizing.font_meta,
                                color: blend_color(
                                    style.accent_warning,
                                    style.text_primary,
                                    style.state_focus_pulse_blend,
                                ),
                                max_width: Some(row_text_layout.index_label.width().max(12.0)),
                                align: TextAlign::Right,
                            },
                        );
                        emit_text(
                            text_runs,
                            TextRun {
                                text: row.label.clone(),
                                position: row_text_layout.sample_label.min,
                                font_size: sizing.font_body,
                                color: blend_color(
                                    style.accent_warning,
                                    style.text_primary,
                                    style.state_focus_pulse_blend,
                                ),
                                max_width: Some(row_text_layout.sample_label.width().max(20.0)),
                                align: TextAlign::Left,
                            },
                        );
                    }
                }
            }
        }

        let tabs = compute_browser_tabs_rects(layout.browser_tabs, sizing);
        let (samples_fill, map_fill, samples_text_color, map_text_color) = if !model.map.active {
            (
                blend_color(
                    style.surface_overlay,
                    style.bg_tertiary,
                    style.state_selected_blend + 0.1,
                ),
                style.surface_base,
                blend_color(
                    style.accent_mint,
                    style.text_primary,
                    style.state_selected_blend,
                ),
                style.text_muted,
            )
        } else {
            (
                style.surface_base,
                blend_color(
                    style.surface_overlay,
                    style.bg_tertiary,
                    style.state_selected_blend + 0.1,
                ),
                style.text_muted,
                blend_color(
                    style.accent_mint,
                    style.text_primary,
                    style.state_selected_blend,
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
        let tabs_text_layout = compute_browser_tabs_text_layout(tabs.samples, tabs.map, sizing);
        let samples_text = format!(
            "{} ({})",
            model.browser_chrome.samples_tab_label,
            model
                .columns
                .get(1)
                .map(|column| column.item_count)
                .unwrap_or(0)
        );
        emit_text(
            text_runs,
            TextRun {
                text: truncate_to_width(
                    &samples_text,
                    tabs_text_layout.samples_label.width().max(40.0),
                    sizing.font_header,
                ),
                position: tabs_text_layout.samples_label.min,
                font_size: sizing.font_header,
                color: samples_text_color,
                max_width: Some(tabs_text_layout.samples_label.width().max(40.0)),
                align: TextAlign::Left,
            },
        );
        emit_text(
            text_runs,
            TextRun {
                text: String::from(model.browser_chrome.map_tab_label.as_str()),
                position: tabs_text_layout.map_label.min,
                font_size: sizing.font_header,
                color: map_text_color,
                max_width: Some(tabs_text_layout.map_label.width().max(40.0)),
                align: TextAlign::Left,
            },
        );

        render_progress_overlay(primitives, text_runs, layout, style, model);
        render_confirm_prompt(primitives, text_runs, layout, style, model);
        render_drag_overlay(primitives, text_runs, layout, style, model);

        frame.clear_color = style.clear_color;
    }

    /// Build only motion-sensitive overlays into reusable buffers.
    pub(crate) fn build_motion_overlay_into(
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

        let lamp_radius = sizing.lamp_radius_base + (motion_wave * sizing.lamp_radius_amp);
        let lamp_color = if self.transport_running {
            style.accent_mint
        } else {
            style.accent_copper
        };
        emit_primitive(
            primitives,
            Primitive::Circle(FillCircle {
                center: Point::new(
                    layout.top_bar.max.x - (sizing.text_inset_x + 14.0),
                    layout.top_bar_title_row.min.y + (layout.top_bar_title_row.height() * 0.5),
                ),
                radius: lamp_radius,
                color: lamp_color,
            }),
        );

        push_waveform_playhead_overlay(primitives, layout, style, model);
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
        let waveform_toolbar_buttons = waveform_toolbar_buttons(layout, style, model);
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
        render_waveform_toolbar_buttons(
            primitives,
            text_runs,
            style,
            sizing,
            &waveform_toolbar_buttons,
        );

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
        );

        frame.clear_color = style.clear_color;
    }

    /// Build a native frame using default style tokens.
    #[allow(dead_code)]
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
    ) {
        if status_right.is_empty() {
            return;
        }
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: layout.status_right_segment,
                color: style.surface_raised,
            }),
        );
        let sizing = style.sizing;
        let status_text_rect =
            compute_status_text_line_rect(layout.status_right_segment, sizing, sizing.font_status);
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

    fn cached_source_row_rects(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
    ) -> &[Rect] {
        let cache_key = sidebar_rows_cache_key(layout, style, model);
        if self.source_row_cache_key != Some(cache_key) {
            self.source_row_rects = rendered_source_row_rects(layout, style, model);
            self.source_row_cache_key = Some(cache_key);
            self.folder_row_cache_key = Some(cache_key);
        }
        &self.source_row_rects
    }

    fn cached_folder_row_rects(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
    ) -> &[Rect] {
        let cache_key = sidebar_rows_cache_key(layout, style, model);
        if self.folder_row_cache_key != Some(cache_key) {
            self.folder_row_rects = rendered_folder_row_rects(layout, style, model);
            self.folder_row_cache_key = Some(cache_key);
            self.source_row_cache_key = Some(cache_key);
        }
        &self.folder_row_rects
    }

    fn cached_browser_rows(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
    ) -> &[CachedBrowserRow] {
        let cache_key = browser_rows_cache_key(layout, style, model);
        let truncation_cache_key = browser_row_truncation_cache_key(layout, style, cache_key);
        if self.browser_row_truncation_cache_key != Some(truncation_cache_key) {
            self.browser_row_truncation_cache.clear();
            self.browser_row_truncation_cache_key = Some(truncation_cache_key);
        }
        self.browser_row_truncation_frame_counts = BrowserRowTruncationFrameCounts::default();
        if self.browser_rows_cache_key != Some(cache_key) {
            self.browser_rows = rendered_browser_rows_cached(
                layout,
                model,
                style,
                &mut self.browser_row_truncation_cache,
                &mut self.browser_row_truncation_frame_counts,
            );
            self.browser_rows_cache_key = Some(cache_key);
        }
        &self.browser_rows
    }
}

/// Return hovered waveform marker x-position for one pointer point.
fn waveform_hover_x_for_point(
    layout: &ShellLayout,
    hover: Option<ShellNodeKind>,
    point: Point,
) -> Option<f32> {
    if hover != Some(ShellNodeKind::WaveformCard) || !layout.waveform_plot.contains(point) {
        return None;
    }
    Some(
        point
            .x
            .clamp(layout.waveform_plot.min.x, layout.waveform_plot.max.x)
            .round(),
    )
}

/// Return one plot-bounded hover marker rectangle for a waveform x-position.
fn waveform_hover_marker_rect(
    waveform_plot: Rect,
    marker_width: f32,
    hover_x: f32,
) -> Option<Rect> {
    if waveform_plot.width() <= 0.0 || waveform_plot.height() <= 0.0 {
        return None;
    }
    let width = marker_width.max(1.0);
    let half = width * 0.5;
    let clamped_x = hover_x.clamp(waveform_plot.min.x, waveform_plot.max.x);
    let left = (clamped_x - half).clamp(waveform_plot.min.x, waveform_plot.max.x - 1.0);
    let right = (left + width).min(waveform_plot.max.x).max(left + 1.0);
    Some(Rect::from_min_max(
        Point::new(left, waveform_plot.min.y),
        Point::new(right, waveform_plot.max.y),
    ))
}

fn map_point_color(style: &StyleTokens, point: &crate::app::MapPointModel) -> Rgba8 {
    if point.focused {
        return style.accent_warning;
    }
    if point.selected {
        return style.accent_mint;
    }
    match point.cluster_id.map(|id| id.rem_euclid(5)) {
        Some(0) => blend_color(style.accent_mint, style.bg_secondary, 0.42),
        Some(1) => blend_color(style.accent_copper, style.bg_secondary, 0.42),
        Some(2) => blend_color(style.accent_warning, style.bg_secondary, 0.42),
        Some(3) => blend_color(style.text_primary, style.bg_secondary, 0.35),
        Some(_) => blend_color(style.text_muted, style.bg_secondary, 0.35),
        None => blend_color(style.text_muted, style.bg_secondary, 0.5),
    }
}

fn map_sample_id_at_point(layout: &ShellLayout, model: &AppModel, point: Point) -> Option<String> {
    if !model.map.active || model.map.points.is_empty() {
        return None;
    }
    let canvas =
        compute_browser_map_canvas_rect(layout.browser_rows, style_for_layout(layout).sizing);
    if !canvas.contains(point) {
        return None;
    }

    let mut best: Option<(f32, &str)> = None;
    for map_point in &model.map.points {
        let center = compute_browser_map_point_center(canvas, map_point.x_milli, map_point.y_milli);
        let radius = if map_point.focused {
            7.0
        } else if map_point.selected {
            6.0
        } else {
            5.0
        };
        let dx = point.x - center.x;
        let dy = point.y - center.y;
        let distance_sq = (dx * dx) + (dy * dy);
        if distance_sq > (radius * radius) {
            continue;
        }
        match best {
            Some((best_distance_sq, _)) if distance_sq >= best_distance_sq => {}
            _ => best = Some((distance_sq, map_point.sample_id.as_str())),
        }
    }
    best.map(|(_, sample_id)| sample_id.to_string())
}

fn update_status_text(model: &AppModel) -> String {
    if !model.update.status_label.is_empty() {
        return model.update.status_label.clone();
    }
    match model.update.status {
        crate::app::UpdateStatusModel::Idle => String::from("Updates: idle"),
        crate::app::UpdateStatusModel::Checking => String::from("Checking updates..."),
        crate::app::UpdateStatusModel::Available => model
            .update
            .available_tag
            .as_deref()
            .map(|tag| format!("Update available: {tag}"))
            .unwrap_or_else(|| String::from("Update available")),
        crate::app::UpdateStatusModel::Error => model
            .update
            .last_error
            .as_deref()
            .map(|err| format!("Update check failed: {err}"))
            .unwrap_or_else(|| String::from("Update check failed")),
    }
}

fn update_hint_text(model: &AppModel) -> String {
    if !model.update.action_hint_label.is_empty() {
        return model.update.action_hint_label.clone();
    }
    match model.update.status {
        crate::app::UpdateStatusModel::Idle => String::from("Action: check"),
        crate::app::UpdateStatusModel::Checking => String::from("Action: waiting"),
        crate::app::UpdateStatusModel::Available => {
            if model.update.available_url.is_some() {
                String::from("Actions: open | install | dismiss")
            } else {
                String::from("Action: dismiss")
            }
        }
        crate::app::UpdateStatusModel::Error => String::from("Action: retry"),
    }
}

fn update_notes_text(model: &AppModel) -> String {
    if !model.update.release_notes_label.is_empty() {
        return model.update.release_notes_label.clone();
    }
    String::new()
}

fn update_action_buttons(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> Vec<ActionButton> {
    let specs: Vec<(&'static str, bool, UiAction)> = match model.update.status {
        crate::app::UpdateStatusModel::Idle => {
            vec![("Check", true, UiAction::CheckForUpdates)]
        }
        crate::app::UpdateStatusModel::Checking => Vec::new(),
        crate::app::UpdateStatusModel::Available => {
            let has_url = model.update.available_url.is_some();
            vec![
                ("Open", has_url, UiAction::OpenUpdateLink),
                ("Install", has_url, UiAction::InstallUpdate),
                ("Dismiss", true, UiAction::DismissUpdate),
            ]
        }
        crate::app::UpdateStatusModel::Error => {
            vec![("Retry", true, UiAction::CheckForUpdates)]
        }
    };
    if specs.is_empty() {
        return Vec::new();
    }
    let labels: Vec<&str> = specs.iter().map(|(label, _, _)| *label).collect();
    let rects = compute_update_action_button_rects(
        layout.top_bar_title_row,
        layout.top_bar_action_cluster,
        style.sizing,
        &labels,
    );
    let start_index = specs.len().saturating_sub(rects.len());
    rects
        .into_iter()
        .zip(specs.into_iter().skip(start_index))
        .map(|(rect, (label, enabled, action))| ActionButton {
            rect,
            label,
            enabled,
            action,
            text_color: style.text_primary,
        })
        .collect()
}

fn browser_toolbar_layout(
    layout: &ShellLayout,
    style: &StyleTokens,
    browser_buttons: &[ActionButton],
) -> BrowserToolbarLayout {
    let action_left = browser_buttons
        .iter()
        .map(|button| button.rect.min.x)
        .min_by(f32::total_cmp)
        .or(Some(
            layout.browser_toolbar.max.x - style.sizing.text_inset_x,
        ));
    let sections =
        compute_browser_toolbar_sections(layout.browser_toolbar, style.sizing, action_left);
    BrowserToolbarLayout {
        search_field: sections.search_field,
        activity_chip: sections.activity_chip,
        sort_chip: sections.sort_chip,
        triage_chips: sections.triage_chips,
    }
}

fn browser_column_chips(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    browser_buttons: &[ActionButton],
) -> Vec<BrowserColumnChip> {
    let toolbar = browser_toolbar_layout(layout, style, browser_buttons);
    toolbar
        .triage_chips
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, rect)| rect.width() > 1.0)
        .map(|(column, rect)| {
            let fallback = match column {
                0 => "Trash",
                1 => "Neutral",
                _ => "Keep",
            };
            let label = model
                .columns
                .get(column)
                .map(|entry| entry.title.as_str())
                .filter(|value| !value.is_empty())
                .unwrap_or(fallback)
                .to_string();
            BrowserColumnChip {
                rect,
                column,
                label,
                item_count: model
                    .columns
                    .get(column)
                    .map(|entry| entry.item_count)
                    .unwrap_or(0),
                selected: model.selected_column.min(2) == column,
            }
        })
        .collect()
}

fn waveform_toolbar_buttons(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &NativeMotionModel,
) -> Vec<WaveformToolbarButton> {
    let specs = [
        (
            "Mono",
            true,
            model.waveform_channel_view == crate::app::WaveformChannelViewModel::Mono,
            Some(UiAction::SetWaveformChannelView { stereo: false }),
            style.text_muted,
        ),
        (
            "Stereo",
            true,
            model.waveform_channel_view == crate::app::WaveformChannelViewModel::Stereo,
            Some(UiAction::SetWaveformChannelView { stereo: true }),
            style.text_muted,
        ),
        (
            "Norm",
            true,
            model.waveform_normalized_audition_enabled,
            Some(UiAction::SetNormalizedAuditionEnabled {
                enabled: !model.waveform_normalized_audition_enabled,
            }),
            style.highlight_cyan,
        ),
        (
            "BPM Snap",
            true,
            model.waveform_bpm_snap_enabled,
            Some(UiAction::SetBpmSnapEnabled {
                enabled: !model.waveform_bpm_snap_enabled,
            }),
            style.highlight_blue,
        ),
        (
            "Tr Snap",
            true,
            model.waveform_transient_snap_enabled,
            Some(UiAction::SetTransientSnapEnabled {
                enabled: !model.waveform_transient_snap_enabled,
            }),
            style.highlight_orange,
        ),
        (
            "Show Tr",
            true,
            model.waveform_transient_markers_enabled,
            Some(UiAction::SetTransientMarkersEnabled {
                enabled: !model.waveform_transient_markers_enabled,
            }),
            style.highlight_blue_soft,
        ),
        (
            "Slice",
            true,
            model.waveform_slice_mode_enabled,
            Some(UiAction::SetSliceModeEnabled {
                enabled: !model.waveform_slice_mode_enabled,
            }),
            style.highlight_cyan_soft,
        ),
        (
            "Loop",
            true,
            model.waveform_loop_enabled,
            Some(UiAction::ToggleLoopPlayback),
            style.highlight_blue,
        ),
        (
            "Stop",
            true,
            !model.transport_running,
            Some(UiAction::HandleEscape),
            style.highlight_orange_soft,
        ),
        (
            "Play",
            true,
            model.transport_running,
            Some(UiAction::ToggleTransport),
            style.highlight_cyan,
        ),
        ("Rec", false, false, None, style.highlight_blue_soft),
    ];
    let labels: Vec<&str> = specs.iter().map(|(label, ..)| *label).collect();
    let cluster = Rect::from_min_max(
        Point::new(
            layout.waveform_header.min.x + (layout.waveform_header.width() * 0.42),
            layout.waveform_header.min.y,
        ),
        layout.waveform_header.max,
    );
    let rects =
        compute_update_action_button_rects(layout.waveform_header, cluster, style.sizing, &labels);
    let start_index = specs.len().saturating_sub(rects.len());
    rects
        .into_iter()
        .zip(specs.into_iter().skip(start_index))
        .map(
            |(rect, (label, enabled, active, action, text_color))| WaveformToolbarButton {
                rect,
                label,
                enabled,
                active,
                action,
                text_color,
            },
        )
        .collect()
}

fn waveform_toolbar_left_edge(buttons: &[WaveformToolbarButton], fallback: f32) -> f32 {
    buttons
        .iter()
        .map(|button| button.rect.min.x)
        .min_by(f32::total_cmp)
        .unwrap_or(fallback)
}

fn render_waveform_toolbar_buttons(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    style: &StyleTokens,
    sizing: SizingTokens,
    buttons: &[WaveformToolbarButton],
) {
    for button in buttons {
        let label_rect = compute_action_button_text_rect(button.rect, sizing);
        let fill = if button.enabled {
            if button.active {
                blend_color(
                    style.surface_overlay,
                    style.bg_tertiary,
                    style.state_selected_blend,
                )
            } else {
                style.surface_overlay
            }
        } else {
            style.control_disabled_fill
        };
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: button.rect,
                color: fill,
            }),
        );
        push_border(
            primitives,
            button.rect,
            if button.enabled && button.active {
                blend_color(style.border_emphasis, style.text_primary, 0.58)
            } else if button.enabled {
                style.border
            } else {
                style.control_disabled_fill
            },
            sizing.border_width,
        );
        emit_text(
            text_runs,
            TextRun {
                text: button.label.to_string(),
                position: label_rect.min,
                font_size: sizing.font_meta,
                color: if button.enabled {
                    button.text_color
                } else {
                    style.text_muted
                },
                max_width: Some(label_rect.width().max(12.0)),
                align: TextAlign::Center,
            },
        );
    }
}

fn top_bar_controls_layout(layout: &ShellLayout, sizing: SizingTokens) -> TopBarControlsLayout {
    let resolved = compute_top_bar_controls_sections(layout, sizing);
    TopBarControlsLayout {
        active: resolved.active,
        options_label: resolved.options_label,
        volume_meter: resolved.volume_meter,
        volume_value: resolved.volume_value,
        volume_label: resolved.volume_label,
    }
}

fn sidebar_sections(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> SidebarSections {
    let resolved = compute_sidebar_row_sections(
        layout.sidebar_rows,
        style.sizing,
        SidebarRowCounts {
            source_rows: rendered_source_rows(style, model),
            folder_rows: rendered_folder_rows(style, model),
        },
    );
    SidebarSections {
        source_rows: resolved.source_rows,
        folder_header: resolved.folder_header,
        folder_rows: resolved.folder_rows,
    }
}

fn browser_action_buttons(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> Vec<ActionButton> {
    let definitions = [
        (
            "Rename",
            model.browser_actions.can_rename,
            UiAction::StartBrowserRename,
            style.text_primary,
        ),
        (
            "Trash",
            model.browser_actions.can_tag,
            UiAction::TagBrowserSelection {
                target: BrowserTagTarget::Trash,
            },
            style.accent_warning,
        ),
        (
            "Neutral",
            model.browser_actions.can_tag,
            UiAction::TagBrowserSelection {
                target: BrowserTagTarget::Neutral,
            },
            style.text_muted,
        ),
        (
            "Keep",
            model.browser_actions.can_tag,
            UiAction::TagBrowserSelection {
                target: BrowserTagTarget::Keep,
            },
            style.accent_mint,
        ),
        (
            "Delete",
            model.browser_actions.can_delete,
            UiAction::DeleteBrowserSelection,
            style.accent_copper,
        ),
    ];
    let rects = compute_browser_action_button_rects(
        layout.browser_toolbar,
        style.sizing,
        definitions.len(),
    );
    let start_index = definitions.len().saturating_sub(rects.len());
    rects
        .into_iter()
        .zip(definitions.into_iter().skip(start_index))
        .map(
            |(rect, (label, enabled, action, text_color))| ActionButton {
                rect,
                label,
                enabled,
                action,
                text_color,
            },
        )
        .collect()
}

fn source_action_buttons(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> Vec<ActionButton> {
    let definitions = [
        (
            "New",
            model.sources.folder_actions.can_create_folder,
            UiAction::StartNewFolder,
            style.text_primary,
        ),
        (
            "Root",
            model.sources.folder_actions.can_create_folder_at_root,
            UiAction::StartNewFolderAtRoot,
            style.text_muted,
        ),
        (
            "Rename",
            model.sources.folder_actions.can_rename_folder,
            UiAction::StartFolderRename,
            style.accent_warning,
        ),
        (
            "Delete",
            model.sources.folder_actions.can_delete_folder,
            UiAction::DeleteFocusedFolder,
            style.accent_copper,
        ),
        (
            "Recovery",
            model.sources.folder_actions.can_clear_recovery_log,
            UiAction::ClearFolderDeleteRecoveryLog,
            style.accent_mint,
        ),
    ];
    let rects =
        compute_sidebar_action_button_rects(layout.sidebar_footer, style.sizing, definitions.len());
    let start_index = definitions.len().saturating_sub(rects.len());
    rects
        .into_iter()
        .zip(definitions.into_iter().skip(start_index))
        .map(
            |(rect, (label, enabled, action, text_color))| ActionButton {
                rect,
                label,
                enabled,
                action,
                text_color,
            },
        )
        .collect()
}

/// Build source context-menu panel geometry and action buttons.
fn source_context_menu_spec(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    menu: Option<SourceContextMenuState>,
) -> Option<(Rect, Vec<ActionButton>)> {
    let menu = menu?;
    if menu.row_index >= model.sources.rows.len() {
        return None;
    }
    let source_index = menu.row_index;
    let definitions = [
        (
            "Reload",
            true,
            UiAction::ReloadSourceRow {
                index: source_index,
            },
            style.text_primary,
        ),
        (
            "Hard sync",
            true,
            UiAction::HardSyncSourceRow {
                index: source_index,
            },
            style.accent_warning,
        ),
        (
            "Open folder",
            true,
            UiAction::OpenSourceFolderRow {
                index: source_index,
            },
            style.accent_mint,
        ),
        (
            "Remove dead links",
            true,
            UiAction::RemoveDeadLinksForSourceRow {
                index: source_index,
            },
            style.accent_copper,
        ),
    ];
    let sizing = style.sizing;
    let panel_padding = sizing.panel_inset.max(4.0);
    let button_width = sizing.sidebar_action_button_width.max(168.0);
    let button_height = sizing.sidebar_action_button_height.max(18.0);
    let button_gap = sizing.sidebar_action_button_gap.max(2.0);
    let button_count = definitions.len();
    let panel_width = button_width + panel_padding * 2.0;
    let panel_height = (button_height * button_count as f32)
        + (button_gap * button_count.saturating_sub(1) as f32)
        + panel_padding * 2.0;
    let min_x = layout.sidebar.min.x + sizing.panel_inset;
    let max_x = (layout.sidebar.max.x - sizing.panel_inset - panel_width).max(min_x);
    let min_y = layout.sidebar.min.y + sizing.panel_inset;
    let max_y = (layout.sidebar.max.y - sizing.panel_inset - panel_height).max(min_y);
    let panel_min = Point::new(
        menu.anchor.x.clamp(min_x, max_x),
        menu.anchor.y.clamp(min_y, max_y),
    );
    let panel_rect = Rect::from_min_max(
        panel_min,
        Point::new(panel_min.x + panel_width, panel_min.y + panel_height),
    );
    let mut buttons = Vec::with_capacity(button_count);
    let button_x = panel_rect.min.x + panel_padding;
    let mut button_y = panel_rect.min.y + panel_padding;
    for (label, enabled, action, text_color) in definitions {
        let rect = Rect::from_min_max(
            Point::new(button_x, button_y),
            Point::new(button_x + button_width, button_y + button_height),
        );
        buttons.push(ActionButton {
            rect,
            label,
            enabled,
            action,
            text_color,
        });
        button_y += button_height + button_gap;
    }
    Some((panel_rect, buttons))
}

#[cfg(test)]
mod tests;
