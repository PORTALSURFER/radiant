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

/// Named state used to build one existing editable tree row.
///
/// This keeps row construction readable as the tree model grows: callers name
/// the structural flags they care about instead of passing a long positional
/// boolean list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EditableTreeRowParts {
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
}

impl EditableTreeRowParts {
    /// Build named editable tree row parts with default structural flags.
    pub fn new(label: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            detail: detail.into(),
            ..Self::default()
        }
    }
}

/// Interaction flags for one editable tree row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EditableTreeRowFlags {
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
}

impl EditableTreeRowFlags {
    fn from_parts(parts: &EditableTreeRowParts) -> Self {
        Self {
            selected: parts.selected,
            focused: parts.focused,
            is_root: parts.is_root,
            has_children: parts.has_children,
            expanded: parts.expanded,
        }
    }
}

enum EditableTreeRowDraftSelection {
    KeepCaret,
    SelectAllOnFocus,
}

impl EditableTreeRowDraftSelection {
    fn select_all_on_focus(&self) -> bool {
        matches!(self, Self::SelectAllOnFocus)
    }
}

/// Inline editor state for create and rename draft rows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EditableTreeRowInput {
    /// Editable input value for inline draft rows.
    pub value: Option<String>,
    /// Placeholder text for inline draft rows.
    pub placeholder: Option<String>,
    /// Validation error for inline draft rows.
    pub error: Option<String>,
    /// Whether the inline draft input should own keyboard focus.
    pub focused: bool,
    /// Whether the next focus transition should select the full input text once.
    pub select_all_on_focus: bool,
}

impl EditableTreeRowInput {
    fn draft(
        value: impl Into<String>,
        placeholder: impl Into<String>,
        error: Option<String>,
        focused: bool,
        selection: EditableTreeRowDraftSelection,
    ) -> Self {
        Self {
            value: Some(value.into()),
            placeholder: Some(placeholder.into()),
            error,
            focused,
            select_all_on_focus: selection.select_all_on_focus(),
        }
    }
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
    /// Interaction and hierarchy flags for existing rows.
    pub flags: EditableTreeRowFlags,
    /// Row kind used for inline draft rendering and hit testing.
    pub kind: EditableRowKind,
    /// Host/controller row index backing this projected row, when applicable.
    pub backing_index: Option<usize>,
    /// Inline input state for create and rename draft rows.
    pub input: EditableTreeRowInput,
}

impl EditableTreeRow {
    /// Build one existing editable tree row from named parts.
    pub fn from_parts(parts: EditableTreeRowParts) -> Self {
        let flags = EditableTreeRowFlags::from_parts(&parts);
        Self {
            label: parts.label,
            detail: parts.detail,
            depth: parts.depth,
            flags,
            kind: EditableRowKind::Existing,
            backing_index: None,
            input: EditableTreeRowInput::default(),
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
            flags: EditableTreeRowFlags::default(),
            kind: EditableRowKind::CreateDraft,
            backing_index: None,
            input: EditableTreeRowInput::draft(
                input_value,
                input_placeholder,
                input_error,
                input_focused,
                EditableTreeRowDraftSelection::KeepCaret,
            ),
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
            flags: EditableTreeRowFlags::default(),
            kind: EditableRowKind::RenameDraft,
            backing_index: None,
            input: EditableTreeRowInput::draft(
                input_value,
                input_placeholder,
                input_error,
                input_focused,
                EditableTreeRowDraftSelection::SelectAllOnFocus,
            ),
        }
    }

    /// Set whether the inline input should select all text the next time it receives focus.
    pub fn with_select_all_on_focus(mut self, select_all_on_focus: bool) -> Self {
        self.input.select_all_on_focus = select_all_on_focus;
        self
    }
}
