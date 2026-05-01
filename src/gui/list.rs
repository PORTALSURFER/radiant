//! Generic list and row state primitives.

use serde::{Deserialize, Serialize};

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
        ColumnSummary, EditableRowKind, EditableTreeActions, EditableTreeRow, RowProcessingState,
        VirtualListWindow, VirtualListWindowRequest, resolve_virtual_list_window,
    };

    #[test]
    fn column_summary_preserves_title_and_count() {
        let column = ColumnSummary::new("Inbox", 42);

        assert_eq!(column.title, "Inbox");
        assert_eq!(column.item_count, 42);
    }

    #[test]
    fn row_processing_state_defaults_to_none() {
        assert_eq!(RowProcessingState::default(), RowProcessingState::None);
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
}
