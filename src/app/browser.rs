//! Browser/list/map-facing models exposed by the `radiant` app contract.

use std::sync::Arc;

pub use crate::gui::list::RowProcessingState as BrowserRowProcessingState;
use crate::gui::retained::RetainedVec;
pub use crate::gui::selection::TriState as BrowserTagState;
/// One clickable tag pill projected into the browser metadata sidebar.
pub type BrowserTagPillModel = crate::gui::badge::SelectablePill<BrowserTagState>;
use serde::{Deserialize, Serialize};

/// Browser playback-age filter chips shown in the native toolbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PlaybackAgeFilterChip {
    /// Samples that have never been played.
    NeverPlayed,
    /// Samples whose last playback was at least 30 days ago.
    OlderThanMonth,
    /// Samples whose last playback was at least 7 days ago but less than 30 days ago.
    OlderThanWeek,
}

/// Visual playback-age buckets derived from sample playback history.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum PlaybackAgeBucket {
    /// Samples played within the last 7 days, including future-skewed timestamps.
    #[default]
    Fresh,
    /// Samples last played at least 7 days ago but less than 30 days ago.
    OlderThanWeek,
    /// Samples last played at least 30 days ago.
    OlderThanMonth,
    /// Samples with no recorded playback timestamp.
    NeverPlayed,
}

/// Summary of browser/list state consumed by the native shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserRowModel {
    /// Visible row index in the filtered browser list.
    pub visible_row: usize,
    /// Display label for the row.
    ///
    /// This text is reference-counted so retained top-level app-model clones
    /// can reuse browser row payloads without copying every row label.
    pub label: Arc<str>,
    /// Triage column index (`0..=2`) that currently owns the row.
    pub column: usize,
    /// Signed keep/trash rating level shown alongside the row label (`-3..=3`).
    pub rating_level: i8,
    /// Visual playback-age bucket used to render the browser row age marker.
    pub playback_age_bucket: PlaybackAgeBucket,
    /// Optional inline metadata label rendered at the right edge of the sample lane.
    ///
    /// Hosts can use this for secondary metadata such as BPM or loop/length tags.
    /// Keep/trash text should usually stay empty because the shell already renders
    /// signed rating state via the right-edge indicator rectangles.
    pub bucket_label: Option<Arc<str>>,
    /// Optional normalized similarity fill amount for the right-edge browser bar.
    ///
    /// This encodes one display strength in the inclusive `[0, 255]` range,
    /// where `0` means an empty bar and `255` means a full bar. Hosts should
    /// populate it only for list modes that explicitly present similarity
    /// ordering relative to an anchor sample.
    pub similarity_display_strength: Option<u8>,
    /// Whether this row is currently selected in multi-selection state.
    pub selected: bool,
    /// Whether this row currently has focus/caret.
    pub focused: bool,
    /// Whether the backing sample file is missing on disk.
    pub missing: bool,
    /// Whether the backing sample is marked as a confirmed keep lock.
    pub locked: bool,
    /// Whether the backing sample is session-marked for later review.
    pub marked: bool,
    /// Transient row-scoped processing state for active batch operations.
    pub processing_state: BrowserRowProcessingState,
}

impl BrowserRowModel {
    /// Build a row model, clamping the column into `0..=2`.
    pub fn new(
        visible_row: usize,
        label: impl Into<String>,
        column: usize,
        selected: bool,
        focused: bool,
    ) -> Self {
        Self {
            visible_row,
            label: Arc::<str>::from(label.into()),
            column: column.min(2),
            rating_level: 0,
            playback_age_bucket: PlaybackAgeBucket::Fresh,
            bucket_label: None,
            similarity_display_strength: None,
            selected,
            focused,
            missing: false,
            locked: false,
            marked: false,
            processing_state: BrowserRowProcessingState::None,
        }
    }

    /// Attach a signed keep/trash rating level for inline row indicators.
    pub fn with_rating_level(mut self, rating_level: i8) -> Self {
        self.rating_level = rating_level.clamp(-3, 3);
        self
    }

    /// Attach the playback-age bucket used for row aging treatment.
    pub fn with_playback_age_bucket(mut self, playback_age_bucket: PlaybackAgeBucket) -> Self {
        self.playback_age_bucket = playback_age_bucket;
        self
    }

    /// Attach an explicit inline metadata label for this row.
    pub fn with_bucket_label(mut self, label: impl Into<String>) -> Self {
        self.bucket_label = Some(Arc::<str>::from(label.into()));
        self
    }

    /// Attach a normalized similarity display strength for the compact row bar.
    ///
    /// Values are clamped into `[0.0, 1.0]` and encoded into the integer-backed
    /// `similarity_display_strength` field so retained app-model snapshots can
    /// keep `Eq` semantics.
    pub fn with_similarity_display_strength(mut self, display_strength: f32) -> Self {
        self.similarity_display_strength =
            Some(Self::encode_similarity_display_strength(display_strength));
        self
    }

    /// Encode one normalized similarity display strength into the stored byte range.
    pub fn encode_similarity_display_strength(display_strength: f32) -> u8 {
        (display_strength.clamp(0.0, 1.0) * 255.0).round() as u8
    }

    /// Decode the stored similarity display strength into a normalized fill amount.
    pub fn similarity_display_strength_ratio(&self) -> Option<f32> {
        self.similarity_display_strength
            .map(|strength| f32::from(strength) / 255.0)
    }

    /// Mark whether the backing sample file is missing on disk.
    pub fn with_missing(mut self, missing: bool) -> Self {
        self.missing = missing;
        self
    }

    /// Mark whether the backing sample should render with the keep-lock highlight.
    pub fn with_locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    /// Mark whether the backing sample should render with the session mark treatment.
    pub fn with_marked(mut self, marked: bool) -> Self {
        self.marked = marked;
        self
    }

    /// Attach a transient row-scoped processing state.
    pub fn with_processing_state(mut self, processing_state: BrowserRowProcessingState) -> Self {
        self.processing_state = processing_state;
        self
    }
}

/// Summary of browser/list state consumed by the native shell.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BrowserPanelModel {
    /// Number of rows currently visible in the browser.
    pub visible_count: usize,
    /// Focused visible row index, if any.
    pub selected_visible_row: Option<usize>,
    /// Whether selection-driven browser autoscroll is currently enabled.
    pub autoscroll: bool,
    /// Requested top visible-row index for manual browser viewport scrolling.
    pub view_start_row: usize,
    /// Number of rows currently in multi-selection.
    pub selected_path_count: usize,
    /// Active browser search query.
    pub search_query: String,
    /// Active rating-filter chip states for levels `-3..=3`, plus `4` for locked keeps.
    pub active_rating_filters: [bool; 8],
    /// Active playback-age filter chip states ordered as `Never`, `Month`, `Week`.
    pub active_playback_age_filters: [bool; 3],
    /// Whether the browser is currently filtering down to only marked rows.
    pub marked_filter_active: bool,
    /// Whether the browser is currently filtering to tag-named rows.
    pub tag_named_filter_active: bool,
    /// Whether the tag-named filter is currently inverted.
    pub tag_named_filter_negated: bool,
    /// Placeholder shown when the browser search query is empty.
    pub search_placeholder: Option<String>,
    /// Whether browser search/filter work is still running in the background.
    pub busy: bool,
    /// Whether the selected source is still hydrating before browser rows can project.
    pub source_loading: bool,
    /// Whether optimistic metadata writes are still pending background persistence.
    pub metadata_pending: bool,
    /// Whether file or folder mutations are still running in the background.
    pub file_op_pending: bool,
    /// Whether the browser is currently showing a similarity-filtered result set.
    pub similarity_filtered: bool,
    /// Whether browser duplicate cleanup mode is currently active.
    pub duplicate_cleanup_active: bool,
    /// Display label for the active browser sort mode.
    pub sort_label: Option<String>,
    /// Display label for the currently active browser tab.
    pub active_tab_label: Option<String>,
    /// Display label for the currently focused sample, when known.
    pub focused_sample_label: Option<String>,
    /// Metadata-tag editor sidebar projection scoped to the list tab.
    pub tag_sidebar: BrowserTagSidebarModel,
    /// Selection anchor in visible-row space.
    pub anchor_visible_row: Option<usize>,
    /// Visible rows rendered by the native browser panel.
    pub rows: RetainedVec<BrowserRowModel>,
}

/// Browser chrome copy used by the native shell toolbar and tab strip.
///
/// This separates rendered UI labels from interaction state so hosts can
/// provide layout-specific wording without hardcoded renderer strings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserChromeModel {
    /// Label for the list tab.
    pub samples_tab_label: String,
    /// Label for the map tab.
    pub map_tab_label: String,
    /// Prefix label shown before active search queries.
    pub search_prefix_label: String,
    /// Placeholder label shown when no search query is active.
    pub search_placeholder: String,
    /// Status label shown when browser background work is idle.
    pub activity_ready_label: String,
    /// Status label shown when browser background work is running.
    pub activity_busy_label: String,
    /// Prefix label shown before active sort order labels.
    pub sort_prefix_label: String,
    /// Label describing the active sort order.
    pub sort_order_label: String,
    /// Label describing similarity mode in the map/header chrome.
    pub similarity_toggle_label: String,
    /// Footer/status label for total browser item counts.
    pub item_count_label: String,
}

impl Default for BrowserChromeModel {
    fn default() -> Self {
        Self {
            samples_tab_label: String::from("Samples"),
            map_tab_label: String::from("Similarity map"),
            search_prefix_label: String::from("Search"),
            search_placeholder: String::from("Search samples (Ctrl+F)"),
            activity_ready_label: String::from("Ready"),
            activity_busy_label: String::from("Filtering"),
            sort_prefix_label: String::from("Sort"),
            sort_order_label: String::from("List order"),
            similarity_toggle_label: String::from("points"),
            item_count_label: String::from("0 items"),
        }
    }
}

/// Browser action availability consumed by the native shell action strip.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BrowserActionsModel {
    /// Whether rename can be started for the focused row.
    pub can_rename: bool,
    /// Whether delete can be applied to focused/selected rows.
    pub can_delete: bool,
    /// Whether tag actions can be applied to focused/selected rows.
    pub can_tag: bool,
    /// Whether the focused browser row can be normalized in place.
    pub can_normalize_focused_sample: bool,
    /// Whether the focused browser row can open the seamless loop-crossfade flow.
    pub can_loop_crossfade_focused_sample: bool,
    /// Whether sticky random navigation mode is currently enabled.
    pub random_navigation_enabled: bool,
    /// Whether browser duplicate cleanup mode is currently enabled.
    pub duplicate_cleanup_active: bool,
    /// Whether the browser-local tag sidebar is currently open.
    pub tag_sidebar_open: bool,
}

/// Browser-local metadata sidebar shown beside the sample list.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BrowserTagSidebarModel {
    /// Whether the sidebar should render in the current browser view.
    pub open: bool,
    /// Count of selected rows represented by the sidebar target set.
    pub selected_count: usize,
    /// Header line describing the current selection/focus context.
    pub header_label: String,
    /// Whether sidebar metadata edits should trigger auto-rename.
    pub auto_rename_enabled: bool,
    /// Current tag search/create input value.
    pub input_value: String,
    /// Placeholder shown for the tag input when empty.
    pub input_placeholder: String,
    /// Exclusive playback-type pills.
    pub playback_type_pills: [BrowserTagPillModel; 2],
    /// Normal tag candidates from common usage or search.
    pub normal_tag_pills: Vec<BrowserTagPillModel>,
    /// Create-new candidate when the input does not exactly match an existing tag.
    pub create_tag_pill: Option<BrowserTagPillModel>,
}

/// Render mode label for the map panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MapRenderModeModel {
    /// Rendered as a density heatmap.
    Heatmap,
    /// Rendered as individual points.
    #[default]
    Points,
}

/// Render data for one map point shown in the native map canvas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapPointModel {
    /// Stable sample id used to route click actions back to the host.
    pub sample_id: Arc<str>,
    /// X position normalized to milli-units (`0..=1000`) across map bounds.
    pub x_milli: u16,
    /// Y position normalized to milli-units (`0..=1000`) across map bounds.
    pub y_milli: u16,
    /// Optional cluster id for color grouping.
    pub cluster_id: Option<i32>,
}

/// Summary of map state consumed by the native shell map tab.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct MapPanelModel {
    /// Whether the map tab is currently active in the browser panel.
    pub active: bool,
    /// Human-readable map summary line.
    pub summary: String,
    /// Legend/status label for map render mode and point density.
    pub legend_label: String,
    /// Selection/focus label for the currently highlighted map sample.
    pub selection_label: String,
    /// Hover label for the currently hovered map sample, when any.
    pub hover_label: String,
    /// Cluster summary label for projected map points.
    pub cluster_label: String,
    /// Viewport label describing zoom/pan state.
    pub viewport_label: String,
    /// Optional error text shown when map data cannot be loaded.
    pub error: Option<String>,
    /// Current map render mode.
    pub render_mode: MapRenderModeModel,
    /// Sample id currently selected in map state, when any.
    pub selected_sample_id: Option<String>,
    /// Sample id currently focused from the browser list, when any.
    pub focused_sample_id: Option<String>,
    /// Points available for rendering in normalized map space.
    pub points: Arc<[MapPointModel]>,
}
