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
        compute_top_bar_controls_text_layout, compute_top_bar_title_text_rect,
        compute_top_bar_update_text_layout, compute_update_action_button_rects,
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
mod frame_build;
mod overlays;
mod svg_icons;
mod waveform_segments;

use self::{browser_rows::*, overlays::*, svg_icons::*, waveform_segments::*};

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
/// Number of animation ticks used for one waveform-toolbar click flash.
const WAVEFORM_TOOLBAR_FLASH_TICKS: u8 = 6;

/// Mutable interaction + animation state for the native shell.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NativeShellState {
    selected_column: usize,
    hovered: Option<ShellNodeKind>,
    hovered_browser_visible_row: Option<usize>,
    hovered_folder_row_index: Option<usize>,
    hovered_waveform_toolbar_hint: Option<WaveformToolbarHoverHint>,
    waveform_toolbar_flash: Option<WaveformToolbarFlash>,
    hovered_waveform_resize_edge: Option<WaveformResizeHoverEdge>,
    waveform_bpm_input_active: bool,
    waveform_bpm_input_display: Option<String>,
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
    /// Secondary inline metadata text in browser rows.
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
    /// Stable digest of waveform tempo label text.
    tempo_label_signature: u64,
    /// Whether waveform BPM editor mode is active.
    bpm_editor_active: bool,
    /// Stable digest of waveform BPM editor display text.
    bpm_editor_display_signature: u64,
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
    waveform_toolbar_flash_active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WaveformToolbarFlash {
    hint: WaveformToolbarHoverHint,
    ticks_remaining: u8,
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

/// Stable hover-target identifier for waveform-toolbar tooltip hints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WaveformToolbarHoverHint {
    /// Mono channel-view toggle.
    Mono,
    /// Split-stereo channel-view toggle.
    Stereo,
    /// Normalized audition toggle.
    NormalizedAudition,
    /// Current playback BPM value display.
    BpmValue,
    /// BPM snap toggle.
    BpmSnap,
    /// Transient snap toggle.
    TransientSnap,
    /// Transient marker visibility toggle.
    ShowTransients,
    /// Slice-mode toggle.
    SliceMode,
    /// Loop playback toggle.
    Loop,
    /// Stop transport action.
    Stop,
    /// Transport toggle action.
    Play,
    /// Record action (currently disabled).
    Record,
}

/// Stable hover target for waveform selection/edit resize edges.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WaveformResizeHoverEdge {
    /// Start edge of the playback selection.
    SelectionStart,
    /// End edge of the playback selection.
    SelectionEnd,
    /// Start edge of the edit selection.
    EditSelectionStart,
    /// End edge of the edit selection.
    EditSelectionEnd,
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
    /// Hovered folder row by rendered sidebar row index.
    pub hovered_folder_row_index: Option<usize>,
    /// Hovered waveform-toolbar hint target.
    pub hovered_waveform_toolbar_hint: Option<WaveformToolbarHoverHint>,
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
    /// Hovered waveform resize-edge target for highlight overlays.
    pub hovered_waveform_resize_edge: Option<WaveformResizeHoverEdge>,
}

/// Compact waveform-motion fingerprint for cursor/playhead overlay caches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WaveformMotionOverlayFingerprint {
    /// Hovered waveform marker x-position bits in shell-space coordinates.
    pub waveform_hover_x_bits: Option<u32>,
    /// Hovered waveform resize-edge target for highlight overlays.
    pub hovered_waveform_resize_edge: Option<WaveformResizeHoverEdge>,
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
    /// Hovered waveform-toolbar icon/button target.
    pub hovered_waveform_toolbar_hint: Option<WaveformToolbarHoverHint>,
    /// Click-flashed waveform-toolbar icon/button target.
    pub flashed_waveform_toolbar_hint: Option<WaveformToolbarHoverHint>,
    /// Remaining flash ticks for waveform-toolbar click feedback.
    pub waveform_toolbar_flash_ticks: u8,
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
        self.transport_running
            || self.startup_frame_tick
            || self.playhead_trail_active
            || self.waveform_toolbar_flash_active
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
            hovered_folder_row_index: None,
            hovered_waveform_toolbar_hint: None,
            waveform_toolbar_flash: None,
            hovered_waveform_resize_edge: None,
            waveform_bpm_input_active: false,
            waveform_bpm_input_display: None,
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
            waveform_toolbar_flash_active: self.waveform_toolbar_flash.is_some(),
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
            .hovered_folder_row_index
            .is_some_and(|row_index| row_index >= model.sources.folder_rows.len())
        {
            self.hovered_folder_row_index = None;
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

    /// Update waveform BPM toolbar editor state used by toolbar rendering.
    pub(crate) fn set_waveform_bpm_editor_state(
        &mut self,
        active: bool,
        display_text: Option<String>,
    ) {
        if self.waveform_bpm_input_active == active
            && self.waveform_bpm_input_display == display_text
        {
            return;
        }
        self.waveform_bpm_input_active = active;
        self.waveform_bpm_input_display = display_text;
        self.waveform_toolbar_hit_test_cache_key = None;
    }

    fn trigger_waveform_toolbar_flash(&mut self, hint: WaveformToolbarHoverHint) {
        self.waveform_toolbar_flash = Some(WaveformToolbarFlash {
            hint,
            ticks_remaining: WAVEFORM_TOOLBAR_FLASH_TICKS,
        });
    }

    /// Return the current state-overlay fingerprint.
    pub(crate) fn state_overlay_fingerprint(&self) -> StateOverlayFingerprint {
        StateOverlayFingerprint {
            selected_column: self.selected_column,
            hovered: self.hovered,
            hovered_browser_visible_row: self.hovered_browser_visible_row,
            hovered_folder_row_index: self.hovered_folder_row_index,
            hovered_waveform_toolbar_hint: self.hovered_waveform_toolbar_hint,
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
            hovered_waveform_resize_edge: self.hovered_waveform_resize_edge,
        }
    }

    /// Return the current waveform-motion overlay fingerprint.
    pub(crate) fn waveform_motion_overlay_fingerprint(&self) -> WaveformMotionOverlayFingerprint {
        WaveformMotionOverlayFingerprint {
            waveform_hover_x_bits: self.waveform_hover_x.map(f32::to_bits),
            hovered_waveform_resize_edge: self.hovered_waveform_resize_edge,
            pulse_phase_bits: self.pulse_phase.to_bits(),
        }
    }

    /// Return the current chrome-motion overlay fingerprint.
    pub(crate) fn chrome_motion_overlay_fingerprint(&self) -> ChromeMotionOverlayFingerprint {
        ChromeMotionOverlayFingerprint {
            transport_running: self.transport_running,
            startup_frame_ticks: self.startup_frame_ticks,
            hovered_waveform_toolbar_hint: self.hovered_waveform_toolbar_hint,
            flashed_waveform_toolbar_hint: self.waveform_toolbar_flash.map(|flash| flash.hint),
            waveform_toolbar_flash_ticks: self
                .waveform_toolbar_flash
                .map_or(0, |flash| flash.ticks_remaining),
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
        if let Some(mut flash) = self.waveform_toolbar_flash {
            flash.ticks_remaining = flash.ticks_remaining.saturating_sub(1);
            self.waveform_toolbar_flash = (flash.ticks_remaining > 0).then_some(flash);
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
        let next_hovered_folder_row =
            self.resolve_hovered_folder_row(layout, model, point, next_hover);
        let next_hovered_waveform_toolbar_hint =
            self.resolve_hovered_waveform_toolbar_hint(layout, model, point, next_hover);
        let next_hovered_waveform_resize_edge =
            hovered_waveform_resize_edge_for_point(layout, model, point, next_hover);
        let next_waveform_hover_x = waveform_hover_x_for_point(layout, next_hover, point);
        let hover_changed = next_hover != self.hovered;
        let browser_row_changed = next_hovered_browser_row != self.hovered_browser_visible_row;
        let folder_row_changed = next_hovered_folder_row != self.hovered_folder_row_index;
        let waveform_toolbar_hint_changed =
            next_hovered_waveform_toolbar_hint != self.hovered_waveform_toolbar_hint;
        let waveform_resize_edge_changed =
            next_hovered_waveform_resize_edge != self.hovered_waveform_resize_edge;
        let waveform_hover_changed =
            next_waveform_hover_x.map(f32::to_bits) != self.waveform_hover_x.map(f32::to_bits);
        if !hover_changed
            && !browser_row_changed
            && !folder_row_changed
            && !waveform_toolbar_hint_changed
            && !waveform_resize_edge_changed
            && !waveform_hover_changed
        {
            return CursorMoveEffect::None;
        }
        self.hovered = next_hover;
        self.hovered_browser_visible_row = next_hovered_browser_row;
        self.hovered_folder_row_index = next_hovered_folder_row;
        self.hovered_waveform_toolbar_hint = next_hovered_waveform_toolbar_hint;
        self.hovered_waveform_resize_edge = next_hovered_waveform_resize_edge;
        self.waveform_hover_x = next_waveform_hover_x;
        if waveform_hover_changed
            && !hover_changed
            && !browser_row_changed
            && !folder_row_changed
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

    /// Return the sidebar-header add-source button rect in tests.
    #[cfg(test)]
    pub(crate) fn source_add_button_rect(&self, layout: &ShellLayout) -> Option<Rect> {
        source_add_button_rect(layout.sidebar_header, style_for_layout(layout).sizing)
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
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        if source_add_button_rect(layout.sidebar_header, style.sizing)
            .is_some_and(|rect| rect.contains(point))
        {
            return Some(UiAction::OpenOptionsMenu);
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
        render_waveform_toolbar_buttons(
            primitives,
            text_runs,
            style,
            sizing,
            &waveform_toolbar_buttons,
            self.hovered_waveform_toolbar_hint,
            self.waveform_toolbar_flash.map(|flash| flash.hint),
            motion_wave,
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
        let sizing = style.sizing;
        let background_rect =
            status_motion_overlay_rect(layout.status_right_segment, sizing.border_width);
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
        let cache_key = waveform_toolbar_hit_test_cache_key(
            layout,
            model,
            self.waveform_bpm_input_active,
            self.waveform_bpm_input_display.as_deref(),
        );
        if self.waveform_toolbar_hit_test_cache_key != Some(cache_key) {
            self.waveform_toolbar_buttons = waveform_toolbar_buttons(
                layout,
                style,
                model,
                self.waveform_bpm_input_active,
                self.waveform_bpm_input_display.as_deref(),
            );
            self.waveform_toolbar_hit_test_cache_key = Some(cache_key);
        }
        &self.waveform_toolbar_buttons
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
    bpm_editor_active: bool,
    bpm_editor_display: Option<&str>,
) -> WaveformToolbarHitTestCacheKey {
    WaveformToolbarHitTestCacheKey {
        waveform_header_min_x: f32_to_bits(layout.waveform_header.min.x),
        waveform_header_min_y: f32_to_bits(layout.waveform_header.min.y),
        waveform_header_max_x: f32_to_bits(layout.waveform_header.max.x),
        waveform_header_max_y: f32_to_bits(layout.waveform_header.max.y),
        ui_scale: f32_to_bits(layout.ui_scale),
        model_flags: waveform_toolbar_model_flags(model),
        tempo_label_signature: waveform_tempo_label_signature(model),
        bpm_editor_active,
        bpm_editor_display_signature: text_signature(bpm_editor_display),
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

fn waveform_tempo_label_signature(model: &NativeMotionModel) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    model.waveform_tempo_label.hash(&mut hasher);
    hasher.finish()
}

fn text_signature(value: Option<&str>) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn waveform_toolbar_hover_hint(label: &str) -> Option<WaveformToolbarHoverHint> {
    match label {
        "Mono" => Some(WaveformToolbarHoverHint::Mono),
        "Stereo" => Some(WaveformToolbarHoverHint::Stereo),
        "Norm" => Some(WaveformToolbarHoverHint::NormalizedAudition),
        "BPM Value" => Some(WaveformToolbarHoverHint::BpmValue),
        "BPM Snap" => Some(WaveformToolbarHoverHint::BpmSnap),
        "Tr Snap" => Some(WaveformToolbarHoverHint::TransientSnap),
        "Show Tr" => Some(WaveformToolbarHoverHint::ShowTransients),
        "Slice" => Some(WaveformToolbarHoverHint::SliceMode),
        "Loop" => Some(WaveformToolbarHoverHint::Loop),
        "Stop" => Some(WaveformToolbarHoverHint::Stop),
        "Play" => Some(WaveformToolbarHoverHint::Play),
        "Rec" => Some(WaveformToolbarHoverHint::Record),
        _ => None,
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

/// Return the hovered waveform resize-edge target for one pointer point.
fn hovered_waveform_resize_edge_for_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    hover: Option<ShellNodeKind>,
) -> Option<WaveformResizeHoverEdge> {
    if hover != Some(ShellNodeKind::WaveformCard) || !layout.waveform_plot.contains(point) {
        return None;
    }
    hovered_resize_edge_for_range(layout, model, point, model.waveform.edit_selection_milli)
        .map(|left_edge| {
            if left_edge {
                WaveformResizeHoverEdge::EditSelectionStart
            } else {
                WaveformResizeHoverEdge::EditSelectionEnd
            }
        })
        .or_else(|| {
            hovered_resize_edge_for_range(layout, model, point, model.waveform.selection_milli).map(
                |left_edge| {
                    if left_edge {
                        WaveformResizeHoverEdge::SelectionStart
                    } else {
                        WaveformResizeHoverEdge::SelectionEnd
                    }
                },
            )
        })
}

/// Return whether the pointer is hovering the start (`true`) or end (`false`) edge of one range.
fn hovered_resize_edge_for_range(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    range: Option<crate::app::NormalizedRangeModel>,
) -> Option<bool> {
    let range = range?;
    let start_milli = range.start_milli.min(range.end_milli);
    let end_milli = range.start_milli.max(range.end_milli);
    if end_milli <= start_milli {
        return None;
    }
    let (handle_top, handle_bottom) = waveform_centered_resize_edge_y_bounds(layout.waveform_plot);
    if point.y < handle_top || point.y > handle_bottom {
        return None;
    }
    let start_x = waveform_x_for_milli(layout.waveform_plot, model, start_milli);
    let end_x = waveform_x_for_milli(layout.waveform_plot, model, end_milli);
    let threshold = 7.0;
    let start_distance = (point.x - start_x).abs();
    let end_distance = (point.x - end_x).abs();
    if start_distance > threshold && end_distance > threshold {
        return None;
    }
    Some(start_distance <= end_distance)
}

/// Convert one normalized waveform milli position into plot-space x.
fn waveform_x_for_milli(plot: Rect, model: &AppModel, milli: u16) -> f32 {
    let view_start = f32::from(model.waveform.view_start_milli.min(1000)) / 1000.0;
    let view_end = f32::from(model.waveform.view_end_milli.min(1000)) / 1000.0;
    let view_width = (view_end - view_start).max(f32::EPSILON);
    let absolute_ratio = f32::from(milli.min(1000)) / 1000.0;
    let ratio_in_view = ((absolute_ratio - view_start) / view_width).clamp(0.0, 1.0);
    plot.min.x + (plot.width() * ratio_in_view)
}

/// Return the centered vertical hit span used by waveform edge-resize targets.
fn waveform_centered_resize_edge_y_bounds(plot: Rect) -> (f32, f32) {
    let height = (plot.height() * 0.34).max(1.0).min(plot.height());
    let center_y = plot.min.y + (plot.height() * 0.5);
    let top = (center_y - (height * 0.5)).max(plot.min.y);
    let bottom = (top + height).min(plot.max.y).max(top + 1.0);
    (top, bottom)
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
    let _ = (layout, style, model, browser_buttons);
    Vec::new()
}

fn waveform_toolbar_buttons(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &NativeMotionModel,
    bpm_input_active: bool,
    bpm_input_display: Option<&str>,
) -> Vec<WaveformToolbarButton> {
    let bpm_value_label = waveform_toolbar_bpm_value_label(model, bpm_input_display);
    let specs = vec![
        (
            "Mono",
            None,
            true,
            model.waveform_channel_view == crate::app::WaveformChannelViewModel::Mono,
            Some(UiAction::SetWaveformChannelView { stereo: false }),
            style.text_muted,
        ),
        (
            "Stereo",
            None,
            true,
            model.waveform_channel_view == crate::app::WaveformChannelViewModel::Stereo,
            Some(UiAction::SetWaveformChannelView { stereo: true }),
            style.text_muted,
        ),
        (
            "Norm",
            None,
            true,
            model.waveform_normalized_audition_enabled,
            Some(UiAction::SetNormalizedAuditionEnabled {
                enabled: !model.waveform_normalized_audition_enabled,
            }),
            style.highlight_cyan,
        ),
        (
            "BPM Value",
            Some(bpm_value_label),
            true,
            bpm_input_active,
            None,
            style.text_primary,
        ),
        (
            "BPM Snap",
            None,
            true,
            model.waveform_bpm_snap_enabled,
            Some(UiAction::SetBpmSnapEnabled {
                enabled: !model.waveform_bpm_snap_enabled,
            }),
            style.highlight_blue,
        ),
        (
            "Tr Snap",
            None,
            true,
            model.waveform_transient_snap_enabled,
            Some(UiAction::SetTransientSnapEnabled {
                enabled: !model.waveform_transient_snap_enabled,
            }),
            style.highlight_orange,
        ),
        (
            "Show Tr",
            None,
            true,
            model.waveform_transient_markers_enabled,
            Some(UiAction::SetTransientMarkersEnabled {
                enabled: !model.waveform_transient_markers_enabled,
            }),
            style.highlight_blue_soft,
        ),
        (
            "Slice",
            None,
            true,
            model.waveform_slice_mode_enabled,
            Some(UiAction::SetSliceModeEnabled {
                enabled: !model.waveform_slice_mode_enabled,
            }),
            style.highlight_cyan_soft,
        ),
        (
            "Loop",
            None,
            true,
            model.waveform_loop_enabled,
            Some(UiAction::ToggleLoopPlayback),
            style.highlight_blue,
        ),
        (
            "Stop",
            None,
            true,
            !model.transport_running,
            Some(UiAction::HandleEscape),
            style.highlight_orange_soft,
        ),
        (
            "Play",
            None,
            true,
            model.transport_running,
            Some(UiAction::ToggleTransport),
            style.highlight_cyan,
        ),
        ("Rec", None, false, false, None, style.highlight_blue_soft),
    ];
    let label_strings: Vec<String> = specs
        .iter()
        .map(|(label, display_text, ..)| waveform_toolbar_layout_label(label, display_text))
        .collect();
    let labels: Vec<&str> = label_strings.iter().map(String::as_str).collect();
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
            |(rect, (label, display_text, enabled, active, action, text_color))| {
                WaveformToolbarButton {
                    rect,
                    label,
                    display_text,
                    enabled,
                    active,
                    action,
                    text_color,
                }
            },
        )
        .collect()
}

fn waveform_toolbar_layout_label(label: &str, display_text: &Option<String>) -> String {
    if label == "BPM Value" {
        return display_text
            .clone()
            .unwrap_or_else(|| String::from("120.0"));
    }
    String::from("Mono")
}

fn waveform_toolbar_bpm_value_label(
    model: &NativeMotionModel,
    bpm_input_display: Option<&str>,
) -> String {
    if let Some(display) = bpm_input_display {
        return display.to_string();
    }
    model
        .waveform_tempo_label
        .as_deref()
        .and_then(parse_waveform_tempo_number_text)
        .unwrap_or_else(|| String::from("120.0"))
}

fn parse_waveform_tempo_number_text(label: &str) -> Option<String> {
    let number = label.split_ascii_whitespace().next()?.trim();
    if number.is_empty() {
        return None;
    }
    let parsed = number.parse::<f32>().ok()?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return None;
    }
    Some(number.to_string())
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
    hovered_hint: Option<WaveformToolbarHoverHint>,
    flashed_hint: Option<WaveformToolbarHoverHint>,
    motion_wave: f32,
) {
    for button in buttons {
        let label_rect = compute_action_button_text_rect(button.rect, sizing);
        let button_hint = waveform_toolbar_hover_hint(button.label);
        let is_hovered = button_hint.is_some() && button_hint == hovered_hint;
        let is_flashed = button_hint.is_some() && button_hint == flashed_hint;
        let icon_color = waveform_toolbar_visual_color(
            style,
            button.text_color,
            button.enabled,
            button.active,
            is_hovered,
            is_flashed,
            motion_wave,
        );
        if let Some(icon) = toolbar_icon_for_button(button) {
            if emit_toolbar_svg_icon(
                primitives,
                icon,
                waveform_toolbar_icon_rect(
                    button.rect,
                    sizing,
                    button.active,
                    is_hovered,
                    is_flashed,
                ),
                icon_color,
            ) {
                continue;
            }
        }
        emit_text(
            text_runs,
            TextRun {
                text: button
                    .display_text
                    .clone()
                    .unwrap_or_else(|| button.label.to_string()),
                position: label_rect.min,
                font_size: sizing.font_meta,
                color: icon_color,
                max_width: Some(label_rect.width().max(12.0)),
                align: TextAlign::Center,
            },
        );
    }
}

fn waveform_toolbar_visual_color(
    style: &StyleTokens,
    base_color: Rgba8,
    enabled: bool,
    active: bool,
    hovered: bool,
    flashed: bool,
    motion_wave: f32,
) -> Rgba8 {
    if !enabled {
        return blend_color(style.text_muted, style.bg_tertiary, 0.42);
    }
    let idle_color = blend_color(style.text_muted, base_color, 0.16);
    let active_color = if active {
        blend_color(base_color, style.text_primary, 0.14 + (motion_wave * 0.08))
    } else {
        idle_color
    };
    let hover_color = if hovered {
        blend_color(active_color, style.text_primary, 0.36)
    } else {
        active_color
    };
    if flashed {
        blend_color(hover_color, style.text_primary, 0.58)
    } else {
        hover_color
    }
}

fn waveform_toolbar_icon_rect(
    button_rect: Rect,
    sizing: SizingTokens,
    active: bool,
    hovered: bool,
    flashed: bool,
) -> Rect {
    let max_side =
        (button_rect.width().min(button_rect.height()) - (sizing.border_width * 4.0)).max(6.0);
    let emphasis = if flashed {
        2.0
    } else if hovered {
        1.0
    } else if active {
        0.6
    } else {
        0.0
    };
    let icon_side = (max_side + emphasis).clamp(8.0, 18.0);
    let offset_x = (button_rect.width() - icon_side).max(0.0) * 0.5;
    let offset_y = (button_rect.height() - icon_side).max(0.0) * 0.5;
    Rect::from_min_max(
        Point::new(button_rect.min.x + offset_x, button_rect.min.y + offset_y),
        Point::new(
            button_rect.min.x + offset_x + icon_side,
            button_rect.min.y + offset_y + icon_side,
        ),
    )
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct BrowserRatingIndicatorLayout {
    pub(super) rects: [Rect; 3],
    pub(super) count: usize,
}

pub(super) fn browser_rating_indicator_reserved_width(
    rating_level: i8,
    sizing: SizingTokens,
) -> f32 {
    let count = rating_level.unsigned_abs().min(3) as usize;
    if count == 0 {
        return 0.0;
    }
    let side = browser_rating_indicator_side(sizing);
    let gap = browser_rating_indicator_gap(sizing);
    let text_gap = browser_rating_indicator_text_gap(sizing);
    (count as f32 * side) + ((count.saturating_sub(1)) as f32 * gap) + text_gap
}

pub(super) fn browser_rating_indicator_layout(
    sample_label: Rect,
    rating_level: i8,
    sizing: SizingTokens,
) -> Option<BrowserRatingIndicatorLayout> {
    let count = rating_level.unsigned_abs().min(3) as usize;
    if count == 0 || sample_label.width() <= 0.0 || sample_label.height() <= 0.0 {
        return None;
    }
    let side = browser_rating_indicator_side(sizing)
        .min(sample_label.width())
        .min(sample_label.height().max(1.0));
    let gap = browser_rating_indicator_gap(sizing);
    let total_width = (count as f32 * side) + ((count.saturating_sub(1)) as f32 * gap);
    let start_x = (sample_label.max.x - total_width).max(sample_label.min.x);
    let min_y = sample_label.min.y + ((sample_label.height() - side) * 0.5).floor();
    let max_y = (min_y + side).min(sample_label.max.y);
    let mut rects = [Rect::from_min_max(sample_label.min, sample_label.min); 3];
    for (index, rect) in rects.iter_mut().take(count).enumerate() {
        let min_x = start_x + index as f32 * (side + gap);
        *rect = Rect::from_min_max(
            Point::new(min_x, min_y),
            Point::new((min_x + side).min(sample_label.max.x), max_y),
        );
    }
    Some(BrowserRatingIndicatorLayout { rects, count })
}

pub(super) fn browser_rating_indicator_color(style: &StyleTokens, rating_level: i8) -> Rgba8 {
    if rating_level < 0 {
        style.accent_trash
    } else {
        style.accent_mint
    }
}

fn browser_rating_indicator_side(sizing: SizingTokens) -> f32 {
    (sizing.font_meta * 0.68).round().clamp(5.0, 8.0)
}

fn browser_rating_indicator_gap(sizing: SizingTokens) -> f32 {
    sizing.border_width.max(1.0) + 1.0
}

fn browser_rating_indicator_text_gap(sizing: SizingTokens) -> f32 {
    sizing.text_inset_x.min(5.0).max(2.0)
}

/// Return width reserved for the inline browser metadata chip cluster plus its left gutter.
pub(super) fn browser_inline_tag_reserved_width(text: &str, sizing: SizingTokens) -> f32 {
    let labels: Vec<&str> = browser_inline_tag_labels(text).collect();
    if labels.is_empty() {
        return 0.0;
    }
    let chips_width: f32 = labels
        .iter()
        .map(|label| browser_inline_tag_chip_width(label, sizing))
        .sum();
    let chip_gap_count = labels.len().saturating_sub(1) as f32;
    chips_width
        + (chip_gap_count * browser_inline_tag_chip_gap(sizing))
        + browser_inline_tag_gap(sizing)
}

/// Approximate the rendered width of one inline browser metadata label.
pub(super) fn browser_inline_tag_text_width(text: &str, sizing: SizingTokens) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    ((text.chars().count() as f32) * (sizing.font_meta * 0.56).max(1.0)).ceil()
}

/// Return the horizontal gap between a sample label and its inline metadata label.
pub(super) fn browser_inline_tag_gap(sizing: SizingTokens) -> f32 {
    sizing.text_inset_x.min(6.0).max(3.0)
}

/// Split one inline browser metadata payload into stable per-chip labels.
pub(super) fn browser_inline_tag_labels(text: &str) -> impl Iterator<Item = &str> + '_ {
    text.split(" · ")
        .map(str::trim)
        .filter(|label| !label.is_empty())
}

/// Return the filled chip width needed for one inline browser metadata label.
pub(super) fn browser_inline_tag_chip_width(text: &str, sizing: SizingTokens) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    browser_inline_tag_text_width(text, sizing) + (browser_inline_tag_chip_padding_x(sizing) * 2.0)
}

/// Compute chip rects for one inline browser metadata cluster.
pub(super) fn browser_inline_tag_chip_rects(
    sample_label: Rect,
    text: &str,
    rating_reserved_width: f32,
    sizing: SizingTokens,
) -> Vec<Rect> {
    if text.is_empty() || sample_label.width() <= 0.0 || sample_label.height() <= 0.0 {
        return Vec::new();
    }
    let labels: Vec<&str> = browser_inline_tag_labels(text).collect();
    if labels.is_empty() {
        return Vec::new();
    }
    let chip_gap = browser_inline_tag_chip_gap(sizing);
    let total_width: f32 = labels
        .iter()
        .map(|label| browser_inline_tag_chip_width(label, sizing))
        .sum::<f32>()
        + (labels.len().saturating_sub(1) as f32 * chip_gap);
    let right_edge = (sample_label.max.x - rating_reserved_width).max(sample_label.min.x);
    let start_x = (right_edge - total_width).max(sample_label.min.x);
    let chip_height = browser_inline_tag_chip_height(sample_label, sizing);
    let min_y = sample_label.min.y + ((sample_label.height() - chip_height) * 0.5).floor();
    let max_y = (min_y + chip_height).min(sample_label.max.y);
    let mut x = start_x;
    labels
        .into_iter()
        .map(|label| {
            let width = browser_inline_tag_chip_width(label, sizing);
            let rect = Rect::from_min_max(
                Point::new(x, min_y),
                Point::new((x + width).min(right_edge), max_y),
            );
            x = (rect.max.x + chip_gap).min(right_edge);
            rect
        })
        .collect()
}

/// Return the inset text origin for one inline browser metadata chip.
pub(super) fn browser_inline_tag_text_origin(chip_rect: Rect, sizing: SizingTokens) -> Point {
    Point::new(
        chip_rect.min.x + browser_inline_tag_chip_padding_x(sizing),
        chip_rect.min.y + ((chip_rect.height() - sizing.font_meta) * 0.5).floor(),
    )
}

fn browser_inline_tag_chip_height(sample_label: Rect, sizing: SizingTokens) -> f32 {
    (sizing.font_meta + (browser_inline_tag_chip_padding_y(sizing) * 2.0))
        .round()
        .clamp(10.0, sample_label.height().max(1.0))
}

fn browser_inline_tag_chip_padding_x(sizing: SizingTokens) -> f32 {
    sizing.text_inset_x.min(5.0).max(3.0)
}

fn browser_inline_tag_chip_padding_y(sizing: SizingTokens) -> f32 {
    sizing.text_inset_y.min(3.0).max(1.0)
}

fn browser_inline_tag_chip_gap(sizing: SizingTokens) -> f32 {
    sizing.border_width.max(1.0) + 2.0
}

/// Return the inset backplate rect used to visually lift the waveform deck.
pub(super) fn waveform_deck_backplate_rect(card_rect: Rect, sizing: SizingTokens) -> Rect {
    if card_rect.width() <= 0.0 || card_rect.height() <= 0.0 {
        return Rect::from_min_max(card_rect.min, card_rect.min);
    }
    let inset = (sizing.panel_inset * 0.55)
        .round()
        .clamp(sizing.border_width + 1.0, 10.0);
    let min = Point::new(
        (card_rect.min.x + inset).min(card_rect.max.x),
        (card_rect.min.y + inset).min(card_rect.max.y),
    );
    let max = Point::new(
        (card_rect.max.x - inset).max(min.x),
        (card_rect.max.y - inset).max(min.y),
    );
    Rect::from_min_max(min, max)
}

fn source_add_button_rect(header_rect: Rect, sizing: SizingTokens) -> Option<Rect> {
    if header_rect.width() <= 0.0 || header_rect.height() <= 0.0 {
        return None;
    }
    let side = (sizing.font_header + (sizing.text_inset_y * 1.5))
        .round()
        .clamp(12.0, header_rect.height().max(12.0));
    if header_rect.width() < side + (sizing.text_inset_x * 2.0) {
        return None;
    }
    let max_x = header_rect.max.x - sizing.text_inset_x.max(0.0);
    let min_x = (max_x - side).max(header_rect.min.x);
    let min_y = header_rect.min.y + ((header_rect.height() - side) * 0.5).floor();
    Some(Rect::from_min_max(
        Point::new(min_x, min_y),
        Point::new(max_x, (min_y + side).min(header_rect.max.y)),
    ))
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
    let _ = (layout, style, model);
    Vec::new()
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
            "Remove source",
            true,
            UiAction::RemoveSourceRow {
                index: source_index,
            },
            style.accent_copper,
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
