//! Browser/list/map-facing models exposed by the `radiant` app contract.

pub use crate::gui::list::ContentListRow as BrowserRowModel;
pub use crate::gui::list::RecencyBucket as PlaybackAgeBucket;
pub use crate::gui::list::RecencyFilterChip as PlaybackAgeFilterChip;
pub use crate::gui::list::RowProcessingState as BrowserRowProcessingState;
use crate::gui::retained::RetainedVec;
pub use crate::gui::selection::TriState as BrowserPillState;
pub use crate::gui::visualization::PointRenderMode as MapRenderModeModel;
pub use crate::gui::visualization::SpatialPanel as MapPanelModel;
pub use crate::gui::visualization::SpatialPoint as MapPointModel;
/// One clickable pill projected into the browser metadata sidebar.
pub type BrowserPillModel = crate::gui::badge::SelectablePill<BrowserPillState>;
/// Browser-local metadata sidebar shown beside the content list.
pub type BrowserPillEditorModel = crate::gui::badge::PillEditorPanel<BrowserPillState>;

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
    /// Display label for the currently focused item, when known.
    pub focused_item_label: Option<String>,
    /// Metadata pill-editor panel projection scoped to the list tab.
    pub pill_editor: BrowserPillEditorModel,
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
    pub items_tab_label: String,
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
            items_tab_label: String::from("Items"),
            map_tab_label: String::from("Map"),
            search_prefix_label: String::from("Search"),
            search_placeholder: String::from("Search items (Ctrl+F)"),
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
    pub can_normalize_focused_item: bool,
    /// Whether the focused browser row can open the seamless loop-crossfade flow.
    pub can_loop_crossfade_focused_item: bool,
    /// Whether sticky random navigation mode is currently enabled.
    pub random_navigation_enabled: bool,
    /// Whether browser duplicate cleanup mode is currently enabled.
    pub duplicate_cleanup_active: bool,
    /// Whether the browser-local pill editor is currently open.
    pub pill_editor_open: bool,
}

impl BrowserPanelModel {
    /// Generic metadata-pill editor projected beside the content list.
    pub fn pill_editor(&self) -> &BrowserPillEditorModel {
        &self.pill_editor
    }
}

impl BrowserActionsModel {
    /// Whether the generic browser pill editor is currently open.
    pub fn pill_editor_open(&self) -> bool {
        self.pill_editor_open
    }
}
