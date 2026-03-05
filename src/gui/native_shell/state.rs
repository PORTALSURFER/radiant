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
    paint::{DrawImage, FillCircle, FillRect, NativeViewFrame, Primitive, TextAlign, TextRun},
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
mod frame_build;
mod overlays;
mod waveform_segments;

use self::{browser_rows::*, overlays::*, waveform_segments::*};

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
const PLAYHEAD_TRAIL_MAX_SAMPLES: usize = 192;
/// Number of overlay frames used to fade one playhead ghost line.
const PLAYHEAD_TRAIL_FADE_FRAMES: u64 = 72;
/// Maximum inserted in-between samples per motion frame for smooth trails.
const PLAYHEAD_TRAIL_MAX_INTERPOLATED_STEPS: usize = 24;
/// Largest contiguous frame delta treated as normal transport motion.
const PLAYHEAD_TRAIL_MAX_CONTIGUOUS_DELTA_MICROS: u64 = 120_000;

/// Mutable interaction + animation state for the native shell.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NativeShellState {
    selected_column: usize,
    hovered: Option<ShellNodeKind>,
    hovered_browser_visible_row: Option<usize>,
    waveform_hover_x: Option<f32>,
    last_waveform_playhead_micros: Option<u32>,
    playhead_trail_samples: Vec<PlayheadTrailSample>,
    playhead_trail_frame_index: u64,
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

/// Invalidation key for browser action/button hit-test geometry caches.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct BrowserActionHitTestCacheKey {
    /// Browser toolbar region minimum x-coordinate.
    browser_toolbar_min_x: u32,
    /// Browser toolbar region minimum y-coordinate.
    browser_toolbar_min_y: u32,
    /// Browser toolbar region maximum x-coordinate.
    browser_toolbar_max_x: u32,
    /// Browser toolbar region maximum y-coordinate.
    browser_toolbar_max_y: u32,
    /// Effective UI scale token bits.
    ui_scale: u32,
    /// Stable digest of action-strip and triage-chip model fields.
    model_signature: u64,
}

/// Invalidation key for waveform-toolbar hit-test geometry caches.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct WaveformToolbarHitTestCacheKey {
    /// Waveform header region minimum x-coordinate.
    waveform_header_min_x: u32,
    /// Waveform header region minimum y-coordinate.
    waveform_header_min_y: u32,
    /// Waveform header region maximum x-coordinate.
    waveform_header_max_x: u32,
    /// Waveform header region maximum y-coordinate.
    waveform_header_max_y: u32,
    /// Effective UI scale token bits.
    ui_scale: u32,
    /// Packed waveform-toolbar model state flags.
    model_flags: u16,
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

/// One retained playhead x-position sample used to build ghost-line trails.
#[derive(Clone, Copy, Debug, PartialEq)]
struct PlayheadTrailSample {
    /// Normalized x-position in `0.0..=1.0`.
    ratio: f32,
    /// Overlay frame index when this sample was captured.
    frame_index: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct NativeAnimationReasons {
    transport_running: bool,
    startup_frame_tick: bool,
    playhead_trail_active: bool,
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
#[cfg(test)]
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

/// Compact waveform-motion fingerprint for cursor/playhead overlay caches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WaveformMotionOverlayFingerprint {
    /// Hovered waveform marker x-position bits in shell-space coordinates.
    pub waveform_hover_x_bits: Option<u32>,
    /// Quantized motion phase to force repaint while dynamic trails fade.
    pub pulse_phase_bits: u32,
}

/// Compact chrome-motion fingerprint for toolbar/tabs/status overlay caches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ChromeMotionOverlayFingerprint {
    /// Whether transport-running animation is active.
    pub transport_running: bool,
    /// Remaining startup animation ticks.
    pub startup_frame_ticks: u8,
    /// Quantized pulse animation phase.
    pub pulse_phase_bits: u32,
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
                .extend(segment_frame.primitives.iter().cloned());
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
        self.transport_running || self.startup_frame_tick || self.playhead_trail_active
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
            last_waveform_playhead_micros: None,
            playhead_trail_samples: Vec::new(),
            playhead_trail_frame_index: 0,
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
    #[cfg(test)]
    pub(crate) fn motion_overlay_fingerprint(&self) -> MotionOverlayFingerprint {
        MotionOverlayFingerprint {
            transport_running: self.transport_running,
            startup_frame_ticks: self.startup_frame_ticks,
            pulse_phase_bits: self.pulse_phase.to_bits(),
            waveform_hover_x_bits: self.waveform_hover_x.map(f32::to_bits),
        }
    }

    /// Return the current waveform-motion overlay fingerprint.
    pub(crate) fn waveform_motion_overlay_fingerprint(&self) -> WaveformMotionOverlayFingerprint {
        WaveformMotionOverlayFingerprint {
            waveform_hover_x_bits: self.waveform_hover_x.map(f32::to_bits),
            pulse_phase_bits: self.pulse_phase.to_bits(),
        }
    }

    /// Return the current chrome-motion overlay fingerprint.
    pub(crate) fn chrome_motion_overlay_fingerprint(&self) -> ChromeMotionOverlayFingerprint {
        ChromeMotionOverlayFingerprint {
            transport_running: self.transport_running,
            startup_frame_ticks: self.startup_frame_ticks,
            pulse_phase_bits: self.pulse_phase.to_bits(),
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
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        let (buttons, chips, toolbar) = self.cached_browser_action_hit_test(layout, &style, model);
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
        self.cached_waveform_toolbar_buttons(layout, &style, motion_model)
            .into_iter()
            .find(|button| button.enabled && button.rect.contains(point))
            .and_then(|button| button.action.clone())
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
        push_waveform_playhead_overlay(primitives, layout, style, model, &playhead_trail_lines);
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
        self.playhead_trail_frame_index = self.playhead_trail_frame_index.saturating_add(1);
        let frame_index = self.playhead_trail_frame_index;
        let previous = self.last_waveform_playhead_micros;
        let current = Self::playhead_position_micros(model);
        self.last_waveform_playhead_micros = current;
        if current.is_none() {
            self.playhead_trail_samples.clear();
            return Vec::new();
        }
        if !model.transport_running {
            self.playhead_trail_samples.clear();
            return Vec::new();
        }
        self.append_playhead_trail_samples_if_moving(
            waveform_plot,
            true,
            previous,
            current,
            frame_index,
        );
        self.prune_playhead_trail_samples(frame_index);
        self.playhead_trail_lines(frame_index)
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
        frame_index: u64,
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
        let pixel_step_ratio = (1.0 / waveform_plot.width().max(1.0)).clamp(0.0005, 0.05);
        let steps = ((delta_ratio.abs() / pixel_step_ratio).ceil() as usize)
            .clamp(1, PLAYHEAD_TRAIL_MAX_INTERPOLATED_STEPS);
        for step in 0..steps {
            let progress = step as f32 / steps as f32;
            let ratio = (previous_ratio + (delta_ratio * progress)).rem_euclid(1.0);
            self.playhead_trail_samples
                .push(PlayheadTrailSample { ratio, frame_index });
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
    fn prune_playhead_trail_samples(&mut self, frame_index: u64) {
        self.playhead_trail_samples.retain(|sample| {
            frame_index.saturating_sub(sample.frame_index) <= PLAYHEAD_TRAIL_FADE_FRAMES
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
    fn playhead_trail_lines(&self, frame_index: u64) -> Vec<PlayheadTrailLine> {
        self.playhead_trail_samples
            .iter()
            .filter_map(|sample| {
                let age = frame_index
                    .saturating_sub(sample.frame_index)
                    .min(PLAYHEAD_TRAIL_FADE_FRAMES);
                let remaining = 1.0 - (age as f32 / PLAYHEAD_TRAIL_FADE_FRAMES as f32);
                let alpha = (0.34 * remaining.powf(1.8)).clamp(0.0, 1.0);
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

    fn cached_browser_action_hit_test(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
    ) -> (&[ActionButton], &[BrowserColumnChip], BrowserToolbarLayout) {
        let cache_key = browser_action_hit_test_cache_key(layout, model);
        if self.browser_action_hit_test_cache_key != Some(cache_key) {
            self.browser_action_buttons = browser_action_buttons(layout, style, model);
            self.browser_column_chips =
                browser_column_chips(layout, style, model, &self.browser_action_buttons);
            self.browser_toolbar_layout = Some(browser_toolbar_layout(
                layout,
                style,
                &self.browser_action_buttons,
            ));
            self.browser_action_hit_test_cache_key = Some(cache_key);
        }
        let toolbar = self
            .browser_toolbar_layout
            .unwrap_or_else(|| browser_toolbar_layout(layout, style, &self.browser_action_buttons));
        (
            &self.browser_action_buttons,
            &self.browser_column_chips,
            toolbar,
        )
    }

    fn cached_waveform_toolbar_buttons(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &NativeMotionModel,
    ) -> &[WaveformToolbarButton] {
        let cache_key = waveform_toolbar_hit_test_cache_key(layout, model);
        if self.waveform_toolbar_hit_test_cache_key != Some(cache_key) {
            self.waveform_toolbar_buttons = waveform_toolbar_buttons(layout, style, model);
            self.waveform_toolbar_hit_test_cache_key = Some(cache_key);
        }
        &self.waveform_toolbar_buttons
    }
}

fn browser_action_hit_test_cache_key(
    layout: &ShellLayout,
    model: &AppModel,
) -> BrowserActionHitTestCacheKey {
    BrowserActionHitTestCacheKey {
        browser_toolbar_min_x: f32_to_bits(layout.browser_toolbar.min.x),
        browser_toolbar_min_y: f32_to_bits(layout.browser_toolbar.min.y),
        browser_toolbar_max_x: f32_to_bits(layout.browser_toolbar.max.x),
        browser_toolbar_max_y: f32_to_bits(layout.browser_toolbar.max.y),
        ui_scale: f32_to_bits(layout.ui_scale),
        model_signature: browser_action_model_signature(model),
    }
}

fn browser_action_model_signature(model: &AppModel) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    model.browser_actions.can_rename.hash(&mut hasher);
    model.browser_actions.can_tag.hash(&mut hasher);
    model.browser_actions.can_delete.hash(&mut hasher);
    model.selected_column.min(2).hash(&mut hasher);
    for index in 0..3 {
        if let Some(column) = model.columns.get(index) {
            column.title.hash(&mut hasher);
            column.item_count.hash(&mut hasher);
        } else {
            index.hash(&mut hasher);
        }
    }
    hasher.finish()
}

fn waveform_toolbar_hit_test_cache_key(
    layout: &ShellLayout,
    model: &NativeMotionModel,
) -> WaveformToolbarHitTestCacheKey {
    WaveformToolbarHitTestCacheKey {
        waveform_header_min_x: f32_to_bits(layout.waveform_header.min.x),
        waveform_header_min_y: f32_to_bits(layout.waveform_header.min.y),
        waveform_header_max_x: f32_to_bits(layout.waveform_header.max.x),
        waveform_header_max_y: f32_to_bits(layout.waveform_header.max.y),
        ui_scale: f32_to_bits(layout.ui_scale),
        model_flags: waveform_toolbar_model_flags(model),
    }
}

fn waveform_toolbar_model_flags(model: &NativeMotionModel) -> u16 {
    let mut bits = 0u16;
    if model.waveform_channel_view == crate::app::WaveformChannelViewModel::Stereo {
        bits |= 1 << 0;
    }
    if model.waveform_normalized_audition_enabled {
        bits |= 1 << 1;
    }
    if model.waveform_bpm_snap_enabled {
        bits |= 1 << 2;
    }
    if model.waveform_transient_snap_enabled {
        bits |= 1 << 3;
    }
    if model.waveform_transient_markers_enabled {
        bits |= 1 << 4;
    }
    if model.waveform_slice_mode_enabled {
        bits |= 1 << 5;
    }
    if model.waveform_loop_enabled {
        bits |= 1 << 6;
    }
    if model.transport_running {
        bits |= 1 << 7;
    }
    bits
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

/// Resolve a stable browser-row border stroke in logical units.
///
/// At `ui_scale == 1.0` this resolves to `1.0` logical px so row borders stay
/// visually consistent at 100% scale.
fn browser_row_border_stroke(layout: &ShellLayout) -> f32 {
    layout.ui_scale.max(1.0)
}

/// Return x-advance reserved for the missing-file marker before a sample label.
fn browser_missing_marker_advance(font_size: f32) -> f32 {
    (font_size * 1.05).max(7.0)
}

/// Snap browser-row border bounds to the border stroke grid to avoid uneven AA
/// widths between top/bottom edges.
fn browser_row_border_rect(rect: Rect, stroke: f32) -> Rect {
    let stroke = stroke.max(1.0);
    let snap = |value: f32| (value / stroke).round() * stroke;
    let min_x = snap(rect.min.x);
    let min_y = snap(rect.min.y);
    let max_x = snap(rect.max.x);
    let max_y = snap(rect.max.y);
    let snapped = Rect::from_min_max(Point::new(min_x, min_y), Point::new(max_x, max_y));
    if snapped.width() <= stroke * 2.0 || snapped.height() <= stroke * 2.0 {
        rect
    } else {
        snapped
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
