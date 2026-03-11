//! Retained native-shell cache keys, counters, and small cache storage types.

use super::*;

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
pub(super) enum BrowserRowTextKind {
    /// Primary sample label text in browser rows.
    Sample,
    /// Secondary inline metadata text in browser rows.
    Bucket,
}

/// Lookup key for one browser-row truncation output.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct BrowserRowTruncationEntryKey {
    /// Stable visible-row identity used to scope cached text.
    pub row_id: u32,
    /// Quantized width bucket used by truncation heuristics.
    pub width_bucket: u16,
    /// Quantized font-size bucket used by truncation heuristics.
    pub font_size_bucket: u16,
    /// Distinguishes sample-label vs bucket-label truncation outputs.
    pub text_kind: BrowserRowTextKind,
}

/// Invalidation key for browser-row truncation cache content.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct BrowserRowTruncationCacheKey {
    /// Browser rows region minimum x-coordinate.
    pub browser_rows_min_x: u32,
    /// Browser rows region minimum y-coordinate.
    pub browser_rows_min_y: u32,
    /// Browser rows region maximum x-coordinate.
    pub browser_rows_max_x: u32,
    /// Browser rows region maximum y-coordinate.
    pub browser_rows_max_y: u32,
    /// Sample-label font size token bits.
    pub font_body_bits: u32,
    /// Bucket-label font size token bits.
    pub font_meta_bits: u32,
    /// Effective UI scale token bits.
    pub ui_scale: u32,
    /// Visible-window row-label content revision fingerprint.
    pub row_text_revision: u64,
}

/// Invalidation key for browser action/button hit-test geometry caches.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct BrowserActionHitTestCacheKey {
    /// Browser toolbar region minimum x-coordinate.
    pub browser_toolbar_min_x: u32,
    /// Browser toolbar region minimum y-coordinate.
    pub browser_toolbar_min_y: u32,
    /// Browser toolbar region maximum x-coordinate.
    pub browser_toolbar_max_x: u32,
    /// Browser toolbar region maximum y-coordinate.
    pub browser_toolbar_max_y: u32,
    /// Effective UI scale token bits.
    pub ui_scale: u32,
    /// Stable digest of action-strip and triage-chip model fields.
    pub model_signature: u64,
}

/// Invalidation key for waveform-toolbar hit-test geometry caches.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct WaveformToolbarHitTestCacheKey {
    /// Waveform header region minimum x-coordinate.
    pub waveform_header_min_x: u32,
    /// Waveform header region minimum y-coordinate.
    pub waveform_header_min_y: u32,
    /// Waveform header region maximum x-coordinate.
    pub waveform_header_max_x: u32,
    /// Waveform header region maximum y-coordinate.
    pub waveform_header_max_y: u32,
    /// Effective UI scale token bits.
    pub ui_scale: u32,
    /// Packed waveform-toolbar model state flags.
    pub model_flags: u16,
    /// Stable digest of waveform tempo label text.
    pub tempo_label_signature: u64,
    /// Whether waveform BPM editor mode is active.
    pub bpm_editor_active: bool,
    /// Stable digest of waveform BPM editor display text.
    pub bpm_editor_display_signature: u64,
}

/// Small retained LRU cache for browser-row text truncation outputs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct BrowserRowTruncationCache {
    pub values: HashMap<BrowserRowTruncationEntryKey, BrowserRowTruncationCacheValue>,
    pub touch_epoch: u64,
}

/// One cached truncation result with the latest logical access epoch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BrowserRowTruncationCacheValue {
    pub truncated: String,
    pub last_touch_epoch: u64,
}

/// Ephemeral sidebar source-menu state tracked by the runtime.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct SourceContextMenuState {
    /// Source row index the menu actions target.
    pub row_index: usize,
    /// Pointer anchor used to place the floating menu panel.
    pub anchor: Point,
}

/// One retained playhead x-position sample used to build ghost-line trails.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PlayheadTrailSample {
    /// Normalized x-position in `0.0..=1.0`.
    pub ratio: f32,
    /// Monotonic animation clock value when this sample was captured.
    pub captured_at_seconds: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct NativeAnimationReasons {
    pub transport_running: bool,
    pub startup_frame_tick: bool,
    pub playhead_trail_active: bool,
    pub waveform_toolbar_flash_active: bool,
    pub source_add_button_flash_active: bool,
    pub status_options_button_flash_active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct WaveformToolbarFlash {
    pub hint: WaveformToolbarHoverHint,
    pub ticks_remaining: u8,
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
    /// Channel-view toggle that swaps between mono and split stereo.
    ChannelView,
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
    /// Active browser-search editor visual signature.
    pub browser_search_editor_signature: u64,
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
    /// Hovered browser rating-filter chip level, if any.
    pub hovered_browser_rating_filter_level: Option<i8>,
    /// Whether the browser search field is hovered.
    pub hovered_browser_search_field: bool,
    /// Whether the source-add button is hovered.
    pub hovered_source_add_button: bool,
    /// Whether the status-bar options button is hovered.
    pub hovered_status_options_button: bool,
    /// Hovered waveform-toolbar icon/button target.
    pub hovered_waveform_toolbar_hint: Option<WaveformToolbarHoverHint>,
    /// Whether the source-add button is currently click-flashed.
    pub flashed_source_add_button: bool,
    /// Remaining flash ticks for source-add-button click feedback.
    pub source_add_button_flash_ticks: u8,
    /// Whether the status-bar options button is currently click-flashed.
    pub flashed_status_options_button: bool,
    /// Remaining flash ticks for status options button click feedback.
    pub status_options_button_flash_ticks: u8,
    /// Click-flashed waveform-toolbar icon/button target.
    pub flashed_waveform_toolbar_hint: Option<WaveformToolbarHoverHint>,
    /// Remaining flash ticks for waveform-toolbar click feedback.
    pub waveform_toolbar_flash_ticks: u8,
    /// Active waveform-BPM editor visual signature.
    pub waveform_bpm_editor_signature: u64,
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
pub(super) trait PrimitiveSink {
    /// Push one primitive into the sink.
    fn push_primitive(&mut self, primitive: Primitive);
}

impl PrimitiveSink for Vec<Primitive> {
    fn push_primitive(&mut self, primitive: Primitive) {
        self.push(primitive);
    }
}

/// Sink for emitted frame text runs.
pub(super) trait TextRunSink {
    /// Push one text run into the sink.
    fn push_text_run(&mut self, text_run: TextRun);
}

impl TextRunSink for Vec<TextRun> {
    fn push_text_run(&mut self, text_run: TextRun) {
        self.push(text_run);
    }
}

/// Emit one primitive into a generic sink.
pub(super) fn emit_primitive(primitives: &mut impl PrimitiveSink, primitive: Primitive) {
    primitives.push_primitive(primitive);
}

/// Emit one text run into a generic sink.
pub(super) fn emit_text(text_runs: &mut impl TextRunSink, text_run: TextRun) {
    text_runs.push_text_run(text_run);
}

/// Shared segmented emit context that routes output into static buckets.
pub(super) struct SegmentedStaticEmitContext<'a> {
    pub layout: &'a ShellLayout,
    pub model: &'a AppModel,
    pub segments: &'a mut StaticFrameSegments,
    pub target_segment: Option<StaticFrameSegment>,
}

/// Primitive sink that routes primitives directly into static buckets.
pub(super) struct SegmentedPrimitiveSink<'a, 'b> {
    pub context: &'a RefCell<SegmentedStaticEmitContext<'b>>,
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
pub(super) struct SegmentedTextRunSink<'a, 'b> {
    pub context: &'a RefCell<SegmentedStaticEmitContext<'b>>,
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
    pub(super) fn needs_animation(self) -> bool {
        self.transport_running
            || self.startup_frame_tick
            || self.playhead_trail_active
            || self.waveform_toolbar_flash_active
            || self.source_add_button_flash_active
            || self.status_options_button_flash_active
    }
}

impl BrowserRowTruncationCache {
    /// Clear all retained truncation entries.
    pub(super) fn clear(&mut self) {
        self.values.clear();
        self.touch_epoch = 0;
    }

    /// Resolve one truncation output from cache or compute and insert on miss.
    pub(super) fn resolve(
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
