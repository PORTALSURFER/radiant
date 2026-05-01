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

#[cfg(test)]
mod tests {
    use super::SelectablePill;
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
}
