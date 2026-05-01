//! Generic form and picker primitives.

/// One selectable option in a picker, menu, or segmented choice list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptionItem<Value> {
    /// Human-readable option label.
    pub label: String,
    /// Whether the option is currently selected.
    pub selected: bool,
    /// Value applied when the option is chosen.
    pub value: Value,
}

/// Overview row for a labeled field and its current value summary.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SummaryField {
    /// Static row label.
    pub label: String,
    /// Current value summary.
    pub value_label: String,
}

#[cfg(test)]
mod tests {
    use super::{OptionItem, SummaryField};

    #[test]
    fn option_item_preserves_label_selection_and_value() {
        let option = OptionItem {
            label: String::from("Default"),
            selected: true,
            value: Some(48_000_u32),
        };

        assert_eq!(option.label, "Default");
        assert!(option.selected);
        assert_eq!(option.value, Some(48_000));
    }

    #[test]
    fn summary_field_defaults_to_empty_text() {
        let field = SummaryField::default();

        assert_eq!(field.label, "");
        assert_eq!(field.value_label, "");
    }
}
