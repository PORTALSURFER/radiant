//! Retained-render segment invalidation contracts exposed by `radiant`.

/// Frame-level feedback from renderer to host bridge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameBuildResult {
    /// Number of generated shape primitives.
    pub primitive_count: usize,
    /// Number of generated text runs.
    pub text_run_count: usize,
    /// Whether runtime should keep animating while idle.
    pub needs_animation: bool,
    /// End-to-end frame time in microseconds for the redraw pass.
    pub frame_total_us: u32,
    /// Present-stage duration in microseconds for the redraw pass.
    pub present_us: u32,
    /// Frame-time budget used to classify redraw jank.
    pub frame_budget_us: u32,
    /// Whether the frame exceeded the configured frame-time budget.
    pub jank: bool,
    /// Whether the redraw produced a successful surface present.
    pub presented: bool,
    /// Whether a present was expected but not completed for this redraw.
    pub missed_present: bool,
}

/// Bitmask describing which projection segments changed during the last model pull.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DirtySegments {
    bits: u16,
}

impl DirtySegments {
    /// Status-bar content segment.
    pub const STATUS_BAR: u16 = 1 << 0;
    /// Browser metadata/chrome segment.
    pub const BROWSER_FRAME: u16 = 1 << 1;
    /// Browser row-window segment.
    pub const BROWSER_ROWS_WINDOW: u16 = 1 << 2;
    /// Map-panel segment.
    pub const MAP_PANEL: u16 = 1 << 3;
    /// Waveform panel/chrome segment.
    pub const WAVEFORM_OVERLAY: u16 = 1 << 4;
    /// Static content that is outside explicit segment buckets.
    pub const GLOBAL_STATIC: u16 = 1 << 5;
    /// State-overlay model fields.
    pub const STATE_OVERLAY: u16 = 1 << 6;
    /// Motion-overlay model fields.
    pub const MOTION_OVERLAY: u16 = 1 << 7;

    const STATIC_MASK: u16 = Self::STATUS_BAR
        | Self::BROWSER_FRAME
        | Self::BROWSER_ROWS_WINDOW
        | Self::MAP_PANEL
        | Self::WAVEFORM_OVERLAY
        | Self::GLOBAL_STATIC;
    const OVERLAY_MASK: u16 = Self::STATE_OVERLAY | Self::MOTION_OVERLAY;

    /// Return an empty segment mask.
    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    /// Return a full segment mask.
    pub const fn all() -> Self {
        Self {
            bits: Self::STATIC_MASK | Self::OVERLAY_MASK,
        }
    }

    /// Construct a segment mask from raw bits.
    pub const fn from_bits(bits: u16) -> Self {
        Self {
            bits: bits & (Self::STATIC_MASK | Self::OVERLAY_MASK),
        }
    }

    /// Return raw bit contents for diagnostics and tests.
    pub const fn bits(self) -> u16 {
        self.bits
    }

    /// Return `true` when the mask contains no segments.
    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }

    /// Return `true` when any static segment requires rebuild.
    pub const fn requires_static_rebuild(self) -> bool {
        (self.bits & Self::STATIC_MASK) != 0
    }

    /// Return `true` when any overlay segment requires rebuild.
    pub const fn requires_overlay_rebuild(self) -> bool {
        (self.bits & Self::OVERLAY_MASK) != 0
    }

    /// Insert one or more segment bits into this mask.
    pub fn insert(&mut self, bits: u16) {
        self.bits |= bits & (Self::STATIC_MASK | Self::OVERLAY_MASK);
    }
}

/// Monotonic revision counters for static projection segments.
///
/// Bridges bump the counters for segments whose projected model slices changed on
/// the most recent `pull_model`. Runtimes use these revisions in retained-scene
/// cache keys to avoid expensive segment hashing on every frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SegmentRevisions {
    /// Status-bar projection revision.
    pub status_bar: u64,
    /// Browser metadata/chrome projection revision.
    pub browser_frame: u64,
    /// Browser visible-row window projection revision.
    pub browser_rows_window: u64,
    /// Map-panel projection revision.
    pub map_panel: u64,
    /// Waveform panel/chrome projection revision.
    pub waveform_overlay: u64,
    /// Global static fields projection revision.
    pub global_static: u64,
}

impl SegmentRevisions {
    /// Return whether any static-segment revision is non-zero.
    pub const fn has_static_revisions(self) -> bool {
        self.status_bar != 0
            || self.browser_frame != 0
            || self.browser_rows_window != 0
            || self.map_panel != 0
            || self.waveform_overlay != 0
            || self.global_static != 0
    }

    /// Bump revisions for the static segments flagged in `dirty_segments`.
    pub fn bump_for_dirty_segments(&mut self, dirty_segments: DirtySegments) {
        let bits = dirty_segments.bits();
        if (bits & DirtySegments::STATUS_BAR) != 0 {
            self.status_bar = self.status_bar.saturating_add(1);
        }
        if (bits & DirtySegments::BROWSER_FRAME) != 0 {
            self.browser_frame = self.browser_frame.saturating_add(1);
        }
        if (bits & DirtySegments::BROWSER_ROWS_WINDOW) != 0 {
            self.browser_rows_window = self.browser_rows_window.saturating_add(1);
        }
        if (bits & DirtySegments::MAP_PANEL) != 0 {
            self.map_panel = self.map_panel.saturating_add(1);
        }
        if (bits & DirtySegments::WAVEFORM_OVERLAY) != 0 {
            self.waveform_overlay = self.waveform_overlay.saturating_add(1);
        }
        if (bits & DirtySegments::GLOBAL_STATIC) != 0 {
            self.global_static = self.global_static.saturating_add(1);
        }
    }
}
