//! Generic list and row state primitives.

use crate::gui::retained::RetainedVec;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Request used to resolve a materialized window for a large logical list.
///
/// The request is item-index based rather than pixel based so host applications
/// can reuse it before projecting widgets or layout nodes. Pixel-based scroll
/// containers should continue to use `layout::VirtualizationPolicy`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VirtualListWindowRequest {
    /// Total logical item count.
    pub total_items: usize,
    /// Number of logical items visible in the viewport.
    pub viewport_len: usize,
    /// Host-requested viewport start.
    pub requested_start: usize,
    /// Extra logical items materialized before and after the viewport.
    pub overscan: usize,
    /// Optional focused item that should stay visible.
    pub focused_index: Option<usize>,
    /// Previously resolved viewport start, used to keep interior focus stable.
    pub previous_start: Option<usize>,
    /// Distance from the viewport edge that triggers focus-follow scrolling.
    pub guard_band: usize,
}

/// Resolved logical window for a virtualized list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VirtualListWindow {
    /// Total logical item count.
    pub total_items: usize,
    /// Start of the visible viewport.
    pub viewport_start: usize,
    /// End of the visible viewport, exclusive.
    pub viewport_end: usize,
    /// Start of the materialized window including overscan.
    pub window_start: usize,
    /// End of the materialized window including overscan, exclusive.
    pub window_end: usize,
}

impl VirtualListWindow {
    /// Number of materialized items in this window.
    pub fn window_len(self) -> usize {
        self.window_end.saturating_sub(self.window_start)
    }

    /// Number of visible viewport items in this window.
    pub fn viewport_len(self) -> usize {
        self.viewport_end.saturating_sub(self.viewport_start)
    }

    /// Return whether the materialized window contains no items.
    pub fn is_empty(self) -> bool {
        self.window_start == self.window_end
    }

    /// Return whether a logical item index is inside the materialized window.
    pub fn contains(self, index: usize) -> bool {
        index >= self.window_start && index < self.window_end
    }
}

/// Resolve an item-index based virtualized list window.
///
/// The algorithm is O(1), clamps every caller-provided bound, and avoids
/// allocating. When `focused_index` is present, the previous viewport start is
/// reused while the focus remains away from the configured guard band; near an
/// edge, the viewport scrolls just enough to keep focus comfortably visible.
pub fn resolve_virtual_list_window(request: VirtualListWindowRequest) -> VirtualListWindow {
    if request.total_items == 0 || request.viewport_len == 0 {
        return VirtualListWindow {
            total_items: request.total_items,
            ..VirtualListWindow::default()
        };
    }

    let viewport_len = request.viewport_len.min(request.total_items);
    let max_start = request.total_items.saturating_sub(viewport_len);
    let mut viewport_start = request
        .previous_start
        .unwrap_or(request.requested_start)
        .min(max_start);

    if let Some(focused_index) = request
        .focused_index
        .filter(|index| *index < request.total_items)
    {
        let guard_band = request.guard_band.min(viewport_len.saturating_sub(1) / 2);
        let viewport_end = viewport_start + viewport_len;
        let lower_guard = viewport_start + guard_band;
        let upper_guard_exclusive = viewport_end.saturating_sub(guard_band);

        if focused_index < lower_guard {
            viewport_start = focused_index.saturating_sub(guard_band).min(max_start);
        } else if focused_index >= upper_guard_exclusive {
            viewport_start = focused_index
                .saturating_add(guard_band + 1)
                .saturating_sub(viewport_len)
                .min(max_start);
        }
    } else {
        viewport_start = request.requested_start.min(max_start);
    }

    let viewport_end = viewport_start + viewport_len;
    let window_start = viewport_start.saturating_sub(request.overscan);
    let window_end = viewport_end
        .saturating_add(request.overscan)
        .min(request.total_items);

    VirtualListWindow {
        total_items: request.total_items,
        viewport_start,
        viewport_end,
        window_start,
        window_end,
    }
}

/// Request used to resolve a materialized row window for a large logical grid.
///
/// The request is item-index based and assumes a dense row-major grid. Hosts can
/// resolve the window before projecting card/tile widgets so large grids avoid
/// allocating or rebuilding off-screen rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VirtualGridWindowRequest {
    /// Total logical item count.
    pub total_items: usize,
    /// Number of grid columns in the current viewport.
    pub columns: usize,
    /// Number of visible grid rows in the viewport.
    pub viewport_rows: usize,
    /// Host-requested viewport start row.
    pub requested_row: usize,
    /// Extra rows materialized before and after the viewport.
    pub overscan_rows: usize,
    /// Optional focused item that should stay visible.
    pub focused_index: Option<usize>,
    /// Previously resolved viewport start row, used to keep interior focus stable.
    pub previous_row: Option<usize>,
    /// Distance from the viewport row edge that triggers focus-follow scrolling.
    pub guard_rows: usize,
}

/// Resolved logical row and item window for a virtualized dense grid.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VirtualGridWindow {
    /// Total logical item count.
    pub total_items: usize,
    /// Number of grid columns used to resolve row boundaries.
    pub columns: usize,
    /// Total logical row count.
    pub total_rows: usize,
    /// Start of the visible viewport row window.
    pub viewport_row_start: usize,
    /// End of the visible viewport row window, exclusive.
    pub viewport_row_end: usize,
    /// Start of the materialized row window including overscan.
    pub window_row_start: usize,
    /// End of the materialized row window including overscan, exclusive.
    pub window_row_end: usize,
    /// First materialized item index.
    pub item_start: usize,
    /// End of materialized item indices, exclusive.
    pub item_end: usize,
}

impl VirtualGridWindow {
    /// Number of materialized rows in this window.
    pub fn window_row_len(self) -> usize {
        self.window_row_end.saturating_sub(self.window_row_start)
    }

    /// Number of visible viewport rows in this window.
    pub fn viewport_row_len(self) -> usize {
        self.viewport_row_end
            .saturating_sub(self.viewport_row_start)
    }

    /// Number of materialized items in this window.
    pub fn item_len(self) -> usize {
        self.item_end.saturating_sub(self.item_start)
    }

    /// Return whether the materialized window contains no items.
    pub fn is_empty(self) -> bool {
        self.item_start == self.item_end
    }

    /// Return whether a logical item index is inside the materialized window.
    pub fn contains_item(self, index: usize) -> bool {
        index >= self.item_start && index < self.item_end
    }
}

/// Resolve an item-index based virtualized grid window.
///
/// The algorithm is O(1), clamps every caller-provided bound, and avoids
/// allocating. Focus-follow behavior mirrors [`resolve_virtual_list_window`],
/// but operates on rows derived from the focused item index.
pub fn resolve_virtual_grid_window(request: VirtualGridWindowRequest) -> VirtualGridWindow {
    if request.total_items == 0 || request.columns == 0 || request.viewport_rows == 0 {
        return VirtualGridWindow {
            total_items: request.total_items,
            columns: request.columns,
            ..VirtualGridWindow::default()
        };
    }

    let total_rows = request.total_items.div_ceil(request.columns);
    let viewport_rows = request.viewport_rows.min(total_rows);
    let max_row = total_rows.saturating_sub(viewport_rows);
    let mut viewport_row_start = request
        .previous_row
        .unwrap_or(request.requested_row)
        .min(max_row);

    if let Some(focused_index) = request
        .focused_index
        .filter(|index| *index < request.total_items)
    {
        let focused_row = focused_index / request.columns;
        let guard_rows = request.guard_rows.min(viewport_rows.saturating_sub(1) / 2);
        let viewport_row_end = viewport_row_start + viewport_rows;
        let lower_guard = viewport_row_start + guard_rows;
        let upper_guard_exclusive = viewport_row_end.saturating_sub(guard_rows);

        if focused_row < lower_guard {
            viewport_row_start = focused_row.saturating_sub(guard_rows).min(max_row);
        } else if focused_row >= upper_guard_exclusive {
            viewport_row_start = focused_row
                .saturating_add(guard_rows + 1)
                .saturating_sub(viewport_rows)
                .min(max_row);
        }
    } else {
        viewport_row_start = request.requested_row.min(max_row);
    }

    let viewport_row_end = viewport_row_start + viewport_rows;
    let window_row_start = viewport_row_start.saturating_sub(request.overscan_rows);
    let window_row_end = viewport_row_end
        .saturating_add(request.overscan_rows)
        .min(total_rows);
    let item_start = window_row_start.saturating_mul(request.columns);
    let item_end = window_row_end
        .saturating_mul(request.columns)
        .min(request.total_items);

    VirtualGridWindow {
        total_items: request.total_items,
        columns: request.columns,
        total_rows,
        viewport_row_start,
        viewport_row_end,
        window_row_start,
        window_row_end,
        item_start,
        item_end,
    }
}

/// Render summary for one titled list or table column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnSummary {
    /// Display label for the column header.
    pub title: String,
    /// Number of rows/items represented by the column.
    pub item_count: usize,
}

impl ColumnSummary {
    /// Build a new column summary.
    pub fn new(title: impl Into<String>, item_count: usize) -> Self {
        Self {
            title: title.into(),
            item_count,
        }
    }
}

/// Generic row projection for selectable, virtualized content lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentListRow {
    /// Visible row index in the filtered list.
    pub visible_row: usize,
    /// Display label for the row.
    ///
    /// This text is reference-counted so retained app-model clones can reuse
    /// row payloads without copying every row label.
    pub label: Arc<str>,
    /// Triage or grouping column index that currently owns the row.
    pub column: usize,
    /// Signed row rating level shown alongside the row label (`-3..=3`).
    pub rating_level: i8,
    /// Visual recency bucket used to render the row age marker.
    pub playback_age_bucket: RecencyBucket,
    /// Optional inline metadata label rendered at the row edge.
    pub bucket_label: Option<Arc<str>>,
    /// Optional normalized relatedness fill amount encoded in the inclusive `0..=255` range.
    pub similarity_display_strength: Option<u8>,
    /// Whether this row is currently selected in multi-selection state.
    pub selected: bool,
    /// Whether this row currently has focus/caret.
    pub focused: bool,
    /// Whether the backing content item is unavailable.
    pub missing: bool,
    /// Whether the backing content item is locked/protected.
    pub locked: bool,
    /// Whether the backing content item is marked for later review.
    pub marked: bool,
    /// Transient row-scoped processing state for active batch operations.
    pub processing_state: RowProcessingState,
}

impl ContentListRow {
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
            playback_age_bucket: RecencyBucket::Fresh,
            bucket_label: None,
            similarity_display_strength: None,
            selected,
            focused,
            missing: false,
            locked: false,
            marked: false,
            processing_state: RowProcessingState::None,
        }
    }

    /// Attach a signed rating level for inline row indicators.
    pub fn with_rating_level(mut self, rating_level: i8) -> Self {
        self.rating_level = rating_level.clamp(-3, 3);
        self
    }

    /// Attach the recency bucket used for row aging treatment.
    pub fn with_playback_age_bucket(mut self, playback_age_bucket: RecencyBucket) -> Self {
        self.playback_age_bucket = playback_age_bucket;
        self
    }

    /// Attach an explicit inline metadata label for this row.
    pub fn with_bucket_label(mut self, label: impl Into<String>) -> Self {
        self.bucket_label = Some(Arc::<str>::from(label.into()));
        self
    }

    /// Attach a normalized relatedness display strength for a compact row bar.
    ///
    /// Values are clamped into `[0.0, 1.0]` and encoded into the integer-backed
    /// `similarity_display_strength` field so retained app-model snapshots can
    /// keep `Eq` semantics.
    pub fn with_similarity_display_strength(mut self, display_strength: f32) -> Self {
        self.similarity_display_strength =
            Some(Self::encode_similarity_display_strength(display_strength));
        self
    }

    /// Encode one normalized relatedness display strength into the stored byte range.
    pub fn encode_similarity_display_strength(display_strength: f32) -> u8 {
        (display_strength.clamp(0.0, 1.0) * 255.0).round() as u8
    }

    /// Decode the stored relatedness display strength into a normalized fill amount.
    pub fn similarity_display_strength_ratio(&self) -> Option<f32> {
        self.similarity_display_strength
            .map(|strength| f32::from(strength) / 255.0)
    }

    /// Mark whether the backing content item is unavailable.
    pub fn with_missing(mut self, missing: bool) -> Self {
        self.missing = missing;
        self
    }

    /// Mark whether the backing content item should render with protected treatment.
    pub fn with_locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    /// Mark whether the backing content item should render with review treatment.
    pub fn with_marked(mut self, marked: bool) -> Self {
        self.marked = marked;
        self
    }

    /// Attach a transient row-scoped processing state.
    pub fn with_processing_state(mut self, processing_state: RowProcessingState) -> Self {
        self.processing_state = processing_state;
        self
    }
}

/// Generic state for a searchable, filterable, virtualized content list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentListPanel<Row, Editor> {
    /// Number of rows currently visible in the list.
    pub visible_count: usize,
    /// Focused visible row index, if any.
    pub selected_visible_row: Option<usize>,
    /// Whether selection-driven list autoscroll is currently enabled.
    pub autoscroll: bool,
    /// Requested top visible-row index for manual viewport scrolling.
    pub view_start_row: usize,
    /// Number of rows currently in multi-selection.
    pub selected_item_count: usize,
    /// Active search query.
    pub search_query: String,
    /// Active signed-rating filter chip states for levels `-3..=3`, plus one protected state.
    pub active_rating_filters: [bool; 8],
    /// Active recency filter chip states ordered by host projection.
    pub active_recency_filters: [bool; 3],
    /// Whether the list is currently filtering down to marked rows.
    pub marked_filter_active: bool,
    /// Whether the list is currently filtering to host-derived label rows.
    pub derived_label_filter_active: bool,
    /// Whether the host-derived label filter is currently inverted.
    pub derived_label_filter_negated: bool,
    /// Placeholder shown when the search query is empty.
    pub search_placeholder: Option<String>,
    /// Whether search/filter work is still running in the background.
    pub busy: bool,
    /// Whether the selected data set is still hydrating before rows can project.
    pub data_loading: bool,
    /// Whether optimistic metadata writes are still pending background persistence.
    pub metadata_pending: bool,
    /// Whether background item mutations are still running.
    pub mutation_pending: bool,
    /// Whether the list is currently showing a relatedness-filtered result set.
    pub similarity_filtered: bool,
    /// Whether duplicate cleanup mode is currently active.
    pub duplicate_cleanup_active: bool,
    /// Display label for the active sort mode.
    pub sort_label: Option<String>,
    /// Display label for the currently active tab.
    pub active_tab_label: Option<String>,
    /// Display label for the currently focused item, when known.
    pub focused_item_label: Option<String>,
    /// Metadata editor panel projection scoped to the list tab.
    pub pill_editor: Editor,
    /// Selection anchor in visible-row space.
    pub anchor_visible_row: Option<usize>,
    /// Visible rows rendered by the content list.
    pub rows: RetainedVec<Row>,
}

impl<Row, Editor> Default for ContentListPanel<Row, Editor>
where
    Editor: Default,
{
    fn default() -> Self {
        Self {
            visible_count: 0,
            selected_visible_row: None,
            autoscroll: false,
            view_start_row: 0,
            selected_item_count: 0,
            search_query: String::new(),
            active_rating_filters: [false; 8],
            active_recency_filters: [false; 3],
            marked_filter_active: false,
            derived_label_filter_active: false,
            derived_label_filter_negated: false,
            search_placeholder: None,
            busy: false,
            data_loading: false,
            metadata_pending: false,
            mutation_pending: false,
            similarity_filtered: false,
            duplicate_cleanup_active: false,
            sort_label: None,
            active_tab_label: None,
            focused_item_label: None,
            pill_editor: Editor::default(),
            anchor_visible_row: None,
            rows: RetainedVec::new(),
        }
    }
}

impl<Row, Editor> ContentListPanel<Row, Editor> {
    /// Whether the host-derived label filter is currently active.
    pub fn derived_label_filter_active(&self) -> bool {
        self.derived_label_filter_active
    }

    /// Whether the host-derived label filter is currently inverted.
    pub fn derived_label_filter_negated(&self) -> bool {
        self.derived_label_filter_negated
    }

    /// Metadata editor projected beside the content list.
    pub fn pill_editor(&self) -> &Editor {
        &self.pill_editor
    }
}

/// Generic action availability for a selected or focused content-list item.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ContentListActions {
    /// Whether rename can be started for the focused row.
    pub can_rename: bool,
    /// Whether delete can be applied to focused or selected rows.
    pub can_delete: bool,
    /// Whether metadata-editor actions can be applied to focused or selected rows.
    pub can_edit_pills: bool,
    /// Whether a host-defined focused-item transform is currently available.
    pub can_process_focused_item: bool,
    /// Whether a host-defined focused-item secondary flow is currently available.
    pub can_open_focused_item_flow: bool,
    /// Whether sticky random navigation mode is currently enabled.
    pub random_navigation_enabled: bool,
    /// Whether duplicate cleanup mode is currently enabled.
    pub duplicate_cleanup_active: bool,
    /// Whether the list-local metadata editor is currently open.
    pub pill_editor_open: bool,
}

impl ContentListActions {
    /// Whether generic pill edits can be applied.
    pub fn can_edit_pills(&self) -> bool {
        self.can_edit_pills
    }

    /// Whether the generic pill editor is currently open.
    pub fn pill_editor_open(&self) -> bool {
        self.pill_editor_open
    }
}

/// Generic recency filter chips for list rows with age-based state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RecencyFilterChip {
    /// Items with no recorded activity timestamp.
    NeverPlayed,
    /// Items whose last activity was at least 30 days ago.
    OlderThanMonth,
    /// Items whose last activity was at least 7 days ago but less than 30 days ago.
    OlderThanWeek,
}

/// Visual recency buckets for list rows with age-based markers.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum RecencyBucket {
    /// Items active within the recent window, including future-skewed timestamps.
    #[default]
    Fresh,
    /// Items last active at least 7 days ago but less than 30 days ago.
    OlderThanWeek,
    /// Items last active at least 30 days ago.
    OlderThanMonth,
    /// Items with no recorded activity timestamp.
    NeverPlayed,
}

/// Kind of row displayed by an editable list or tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum EditableRowKind {
    /// Standard existing row projected from host state.
    #[default]
    Existing,
    /// Inline draft row used while creating a new item in place.
    CreateDraft,
    /// Inline draft row used while renaming an existing item in place.
    RenameDraft,
}

/// Action availability for an editable tree or nested list surface.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct EditableTreeActions {
    /// Whether creating a child item under the focused parent is allowed.
    pub can_create_child: bool,
    /// Whether creating an item at the root of the editable tree is allowed.
    pub can_create_root: bool,
    /// Whether renaming the focused item is allowed.
    pub can_rename: bool,
    /// Whether deleting the focused item is allowed.
    pub can_delete: bool,
    /// Whether explicit restore for retained deletes is allowed.
    pub can_restore_retained: bool,
    /// Whether explicit purge for retained deletes is allowed.
    pub can_purge_retained: bool,
    /// Whether clearing the action history is allowed.
    pub can_clear_history: bool,
}

/// Render data for one row in an editable tree or nested list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditableTreeRow {
    /// Display label for the row.
    pub label: String,
    /// Optional secondary detail text for the row.
    pub detail: String,
    /// Tree depth used for indentation.
    pub depth: usize,
    /// Whether this row is currently selected.
    pub selected: bool,
    /// Whether this row currently has keyboard focus.
    pub focused: bool,
    /// Whether this row represents the synthetic root item.
    pub is_root: bool,
    /// Whether this row has child items.
    pub has_children: bool,
    /// Whether this row is expanded in the tree.
    pub expanded: bool,
    /// Row kind used for inline draft rendering and hit testing.
    pub kind: EditableRowKind,
    /// Host/controller row index backing this projected row, when applicable.
    pub backing_index: Option<usize>,
    /// Editable input value for inline draft rows.
    pub input_value: Option<String>,
    /// Placeholder text for inline draft rows.
    pub input_placeholder: Option<String>,
    /// Validation error for inline draft rows.
    pub input_error: Option<String>,
    /// Whether the inline draft input should own keyboard focus.
    pub input_focused: bool,
    /// Whether the next focus transition should select the full input text once.
    pub select_all_on_focus: bool,
}

impl EditableTreeRow {
    /// Build a new editable tree row.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        label: impl Into<String>,
        detail: impl Into<String>,
        depth: usize,
        selected: bool,
        focused: bool,
        is_root: bool,
        has_children: bool,
        expanded: bool,
    ) -> Self {
        Self {
            label: label.into(),
            detail: detail.into(),
            depth,
            selected,
            focused,
            is_root,
            has_children,
            expanded,
            kind: EditableRowKind::Existing,
            backing_index: None,
            input_value: None,
            input_placeholder: None,
            input_error: None,
            input_focused: false,
            select_all_on_focus: false,
        }
    }

    /// Attach the host/controller row index for one existing row.
    pub fn with_backing_index(mut self, backing_index: usize) -> Self {
        self.backing_index = Some(backing_index);
        self
    }

    /// Build one inline create-draft row embedded in the tree.
    pub fn create_draft(
        depth: usize,
        input_value: impl Into<String>,
        input_placeholder: impl Into<String>,
        input_error: Option<String>,
        input_focused: bool,
    ) -> Self {
        Self {
            label: String::new(),
            detail: String::new(),
            depth,
            selected: false,
            focused: false,
            is_root: false,
            has_children: false,
            expanded: false,
            kind: EditableRowKind::CreateDraft,
            backing_index: None,
            input_value: Some(input_value.into()),
            input_placeholder: Some(input_placeholder.into()),
            input_error,
            input_focused,
            select_all_on_focus: false,
        }
    }

    /// Build one inline rename-draft row embedded in the tree.
    pub fn rename_draft(
        depth: usize,
        input_value: impl Into<String>,
        input_placeholder: impl Into<String>,
        input_error: Option<String>,
        input_focused: bool,
    ) -> Self {
        let input_value = input_value.into();
        Self {
            label: input_value.clone(),
            detail: String::new(),
            depth,
            selected: false,
            focused: false,
            is_root: false,
            has_children: false,
            expanded: false,
            kind: EditableRowKind::RenameDraft,
            backing_index: None,
            input_value: Some(input_value),
            input_placeholder: Some(input_placeholder.into()),
            input_error,
            input_focused,
            select_all_on_focus: true,
        }
    }

    /// Set whether the inline input should select all text the next time it receives focus.
    pub fn with_select_all_on_focus(mut self, select_all_on_focus: bool) -> Self {
        self.select_all_on_focus = select_all_on_focus;
        self
    }
}

/// Transient state for row-scoped batch operations.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum RowProcessingState {
    /// The row is not part of an active row-scoped operation.
    #[default]
    None,
    /// The row is waiting in the current batch.
    Queued,
    /// The row is currently being processed.
    Active,
    /// The row completed successfully.
    Completed,
    /// The row was skipped by the batch.
    Skipped,
    /// The row failed during processing.
    Failed,
}

#[cfg(test)]
mod tests {
    use super::{
        ColumnSummary, ContentListActions, ContentListPanel, ContentListRow, EditableRowKind,
        EditableTreeActions, EditableTreeRow, RowProcessingState, VirtualGridWindow,
        VirtualGridWindowRequest, VirtualListWindow, VirtualListWindowRequest,
        resolve_virtual_grid_window, resolve_virtual_list_window,
    };

    #[test]
    fn column_summary_preserves_title_and_count() {
        let column = ColumnSummary::new("Inbox", 42);

        assert_eq!(column.title, "Inbox");
        assert_eq!(column.item_count, 42);
    }

    #[test]
    fn content_list_row_clamps_column_and_relatedness_strength() {
        let row = ContentListRow::new(3, "Item", 99, true, false)
            .with_rating_level(9)
            .with_bucket_label("detail")
            .with_similarity_display_strength(1.5)
            .with_missing(true)
            .with_locked(true)
            .with_marked(true)
            .with_processing_state(RowProcessingState::Queued);

        assert_eq!(row.visible_row, 3);
        assert_eq!(row.label.as_ref(), "Item");
        assert_eq!(row.column, 2);
        assert_eq!(row.rating_level, 3);
        assert_eq!(row.bucket_label.as_deref(), Some("detail"));
        assert_eq!(row.similarity_display_strength, Some(255));
        assert_eq!(row.similarity_display_strength_ratio(), Some(1.0));
        assert!(row.selected);
        assert!(!row.focused);
        assert!(row.missing);
        assert!(row.locked);
        assert!(row.marked);
        assert_eq!(row.processing_state, RowProcessingState::Queued);
    }

    #[test]
    fn row_processing_state_defaults_to_none() {
        assert_eq!(RowProcessingState::default(), RowProcessingState::None);
    }

    #[test]
    fn content_list_panel_defaults_to_empty_generic_projection() {
        let panel: ContentListPanel<ContentListRow, String> = ContentListPanel::default();

        assert_eq!(panel.visible_count, 0);
        assert_eq!(panel.selected_visible_row, None);
        assert!(!panel.derived_label_filter_active());
        assert!(!panel.derived_label_filter_negated());
        assert_eq!(panel.pill_editor(), "");
        assert!(panel.rows.is_empty());
    }

    #[test]
    fn content_list_actions_default_to_unavailable() {
        let actions = ContentListActions::default();

        assert!(!actions.can_rename);
        assert!(!actions.can_delete);
        assert!(!actions.can_edit_pills());
        assert!(!actions.can_process_focused_item);
        assert!(!actions.can_open_focused_item_flow);
        assert!(!actions.pill_editor_open());
    }

    #[test]
    fn editable_row_kind_defaults_to_existing() {
        assert_eq!(EditableRowKind::default(), EditableRowKind::Existing);
    }

    #[test]
    fn editable_tree_actions_default_to_unavailable() {
        let actions = EditableTreeActions::default();

        assert!(!actions.can_create_child);
        assert!(!actions.can_create_root);
        assert!(!actions.can_rename);
        assert!(!actions.can_delete);
        assert!(!actions.can_restore_retained);
        assert!(!actions.can_purge_retained);
        assert!(!actions.can_clear_history);
    }

    #[test]
    fn editable_tree_row_preserves_existing_and_draft_state() {
        let existing = EditableTreeRow::new("Root", "3 items", 0, true, false, true, true, true)
            .with_backing_index(7);
        let draft = EditableTreeRow::rename_draft(1, "Draft", "Name", None, true);

        assert_eq!(existing.label, "Root");
        assert_eq!(existing.detail, "3 items");
        assert_eq!(existing.kind, EditableRowKind::Existing);
        assert_eq!(existing.backing_index, Some(7));
        assert_eq!(draft.kind, EditableRowKind::RenameDraft);
        assert_eq!(draft.input_value.as_deref(), Some("Draft"));
        assert!(draft.input_focused);
        assert!(draft.select_all_on_focus);
    }

    #[test]
    fn virtual_list_window_clamps_requested_bounds_and_applies_overscan() {
        let window = resolve_virtual_list_window(VirtualListWindowRequest {
            total_items: 100,
            viewport_len: 10,
            requested_start: 95,
            overscan: 3,
            ..VirtualListWindowRequest::default()
        });

        assert_eq!(
            window,
            VirtualListWindow {
                total_items: 100,
                viewport_start: 90,
                viewport_end: 100,
                window_start: 87,
                window_end: 100,
            }
        );
        assert_eq!(window.viewport_len(), 10);
        assert_eq!(window.window_len(), 13);
        assert!(window.contains(99));
        assert!(!window.contains(86));
    }

    #[test]
    fn virtual_list_window_keeps_interior_focus_stable() {
        let window = resolve_virtual_list_window(VirtualListWindowRequest {
            total_items: 1_000,
            viewport_len: 20,
            requested_start: 300,
            previous_start: Some(300),
            focused_index: Some(310),
            guard_band: 4,
            ..VirtualListWindowRequest::default()
        });

        assert_eq!(window.viewport_start, 300);
        assert_eq!(window.viewport_end, 320);
    }

    #[test]
    fn virtual_list_window_scrolls_when_focus_reaches_guard_band() {
        let top = resolve_virtual_list_window(VirtualListWindowRequest {
            total_items: 1_000,
            viewport_len: 20,
            requested_start: 300,
            previous_start: Some(300),
            focused_index: Some(302),
            guard_band: 4,
            ..VirtualListWindowRequest::default()
        });
        let bottom = resolve_virtual_list_window(VirtualListWindowRequest {
            total_items: 1_000,
            viewport_len: 20,
            requested_start: 300,
            previous_start: Some(300),
            focused_index: Some(318),
            guard_band: 4,
            ..VirtualListWindowRequest::default()
        });

        assert_eq!(top.viewport_start, 298);
        assert_eq!(bottom.viewport_start, 303);
    }

    #[test]
    fn virtual_list_window_handles_empty_or_zero_viewport_requests() {
        assert!(resolve_virtual_list_window(VirtualListWindowRequest::default()).is_empty());
        assert!(
            resolve_virtual_list_window(VirtualListWindowRequest {
                total_items: 10,
                viewport_len: 0,
                ..VirtualListWindowRequest::default()
            })
            .is_empty()
        );
    }

    #[test]
    fn virtual_grid_window_clamps_rows_and_applies_overscan() {
        let window = resolve_virtual_grid_window(VirtualGridWindowRequest {
            total_items: 103,
            columns: 5,
            viewport_rows: 4,
            requested_row: 99,
            overscan_rows: 2,
            ..VirtualGridWindowRequest::default()
        });

        assert_eq!(
            window,
            VirtualGridWindow {
                total_items: 103,
                columns: 5,
                total_rows: 21,
                viewport_row_start: 17,
                viewport_row_end: 21,
                window_row_start: 15,
                window_row_end: 21,
                item_start: 75,
                item_end: 103,
            }
        );
        assert_eq!(window.viewport_row_len(), 4);
        assert_eq!(window.window_row_len(), 6);
        assert_eq!(window.item_len(), 28);
        assert!(window.contains_item(102));
        assert!(!window.contains_item(74));
    }

    #[test]
    fn virtual_grid_window_keeps_interior_focus_stable() {
        let window = resolve_virtual_grid_window(VirtualGridWindowRequest {
            total_items: 1_000,
            columns: 4,
            viewport_rows: 10,
            requested_row: 40,
            previous_row: Some(40),
            focused_index: Some(178),
            guard_rows: 2,
            ..VirtualGridWindowRequest::default()
        });

        assert_eq!(window.viewport_row_start, 40);
        assert_eq!(window.viewport_row_end, 50);
    }

    #[test]
    fn virtual_grid_window_scrolls_when_focus_reaches_guard_row() {
        let top = resolve_virtual_grid_window(VirtualGridWindowRequest {
            total_items: 1_000,
            columns: 4,
            viewport_rows: 10,
            requested_row: 40,
            previous_row: Some(40),
            focused_index: Some(164),
            guard_rows: 2,
            ..VirtualGridWindowRequest::default()
        });
        let bottom = resolve_virtual_grid_window(VirtualGridWindowRequest {
            total_items: 1_000,
            columns: 4,
            viewport_rows: 10,
            requested_row: 40,
            previous_row: Some(40),
            focused_index: Some(192),
            guard_rows: 2,
            ..VirtualGridWindowRequest::default()
        });

        assert_eq!(top.viewport_row_start, 39);
        assert_eq!(bottom.viewport_row_start, 41);
    }

    #[test]
    fn virtual_grid_window_handles_empty_zero_column_or_zero_viewport_requests() {
        assert!(resolve_virtual_grid_window(VirtualGridWindowRequest::default()).is_empty());
        assert!(
            resolve_virtual_grid_window(VirtualGridWindowRequest {
                total_items: 10,
                columns: 0,
                viewport_rows: 2,
                ..VirtualGridWindowRequest::default()
            })
            .is_empty()
        );
        assert!(
            resolve_virtual_grid_window(VirtualGridWindowRequest {
                total_items: 10,
                columns: 3,
                viewport_rows: 0,
                ..VirtualGridWindowRequest::default()
            })
            .is_empty()
        );
    }
}
