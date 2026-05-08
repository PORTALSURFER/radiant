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
