//! Public badge and pill DTOs.

/// Selectable badge/pill model with host-chosen state semantics.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SelectablePill<State> {
    /// Stable identifier for hit testing and automation.
    pub id: String,
    /// User-facing pill label.
    pub label: String,
    /// Selection value for the current target set.
    pub state: State,
}

/// Visibility and high-level status for a pill editor panel.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PillEditorStatus {
    /// Whether the panel should render in the current view.
    pub open: bool,
    /// Count of selected rows or items represented by the panel target set.
    pub selected_count: usize,
    /// Header line describing the current selection/focus context.
    pub header_label: String,
    /// Whether the host-defined primary side effect is enabled.
    pub primary_action_enabled: bool,
}

/// Text-editing state for a pill editor panel.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PillEditorInput {
    /// Current search/create input value.
    pub input_value: String,
    /// Placeholder shown for the input when empty.
    pub input_placeholder: String,
    /// Whether the input currently owns text-editing focus.
    pub input_focused: bool,
    /// Caret position measured in Unicode scalar values from the start.
    pub input_caret: usize,
    /// Selected text range measured in Unicode scalar values, when any.
    pub input_selection: Option<(usize, usize)>,
}

/// Grouped selectable pill choices for a pill editor panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PillEditorChoices<State> {
    /// Exclusive or high-priority pill choices.
    pub exclusive_pills: [SelectablePill<State>; 2],
    /// Accepted pills that are already applied to the represented target set.
    pub accepted_pills: Vec<SelectablePill<State>>,
    /// Normal pill candidates from common usage or search.
    pub option_pills: Vec<SelectablePill<State>>,
    /// Create-new candidate when the input does not exactly match an existing option.
    pub create_pill: Option<SelectablePill<State>>,
}

impl<State> Default for PillEditorChoices<State>
where
    State: Default,
{
    fn default() -> Self {
        Self {
            exclusive_pills: [SelectablePill::default(), SelectablePill::default()],
            accepted_pills: Vec::new(),
            option_pills: Vec::new(),
            create_pill: None,
        }
    }
}

/// Generic pill-editor panel with a text input and grouped selectable pills.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PillEditorPanel<State> {
    /// Visibility and high-level status.
    pub status: PillEditorStatus,
    /// Text-editing state for search or creation.
    pub input: PillEditorInput,
    /// Grouped selectable pill choices.
    pub choices: PillEditorChoices<State>,
}

impl<State> Default for PillEditorPanel<State>
where
    State: Default,
{
    fn default() -> Self {
        Self {
            status: PillEditorStatus::default(),
            input: PillEditorInput::default(),
            choices: PillEditorChoices::default(),
        }
    }
}
