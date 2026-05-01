//! Generic badge and pill primitives.

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

/// Generic pill-editor panel with a text input and grouped selectable pills.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PillEditorPanel<State> {
    /// Whether the panel should render in the current view.
    pub open: bool,
    /// Count of selected rows or items represented by the panel target set.
    pub selected_count: usize,
    /// Header line describing the current selection/focus context.
    pub header_label: String,
    /// Whether the host-defined primary side effect is enabled.
    pub primary_action_enabled: bool,
    /// Current search/create input value.
    pub input_value: String,
    /// Placeholder shown for the input when empty.
    pub input_placeholder: String,
    /// Exclusive or high-priority pill choices.
    pub exclusive_pills: [SelectablePill<State>; 2],
    /// Normal pill candidates from common usage or search.
    pub option_pills: Vec<SelectablePill<State>>,
    /// Create-new candidate when the input does not exactly match an existing option.
    pub create_pill: Option<SelectablePill<State>>,
}

#[cfg(test)]
mod tests {
    use super::{PillEditorPanel, SelectablePill};
    use crate::gui::selection::TriState;

    #[test]
    fn selectable_pill_preserves_identity_label_and_state() {
        let pill = SelectablePill {
            id: String::from("priority"),
            label: String::from("Priority"),
            state: TriState::Mixed,
        };

        assert_eq!(pill.id, "priority");
        assert_eq!(pill.label, "Priority");
        assert_eq!(pill.state, TriState::Mixed);
    }

    #[test]
    fn pill_editor_panel_defaults_to_closed_empty_panel() {
        let panel: PillEditorPanel<TriState> = PillEditorPanel::default();

        assert!(!panel.open);
        assert_eq!(panel.selected_count, 0);
        assert_eq!(panel.header_label, "");
        assert!(!panel.primary_action_enabled);
        assert_eq!(panel.input_value, "");
        assert_eq!(panel.input_placeholder, "");
        assert_eq!(panel.exclusive_pills.len(), 2);
        assert!(panel.option_pills.is_empty());
        assert!(panel.create_pill.is_none());
    }
}
