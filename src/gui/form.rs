//! Generic form and picker primitives.

mod numeric;
mod paired;

pub use numeric::{
    DecimalTextInputPolicy, parse_finite_decimal_text, rounded_scaled_u16,
    sanitize_decimal_text_insert,
};
pub use paired::{PairedPickerTarget, PairedPickerValue, PairedStatusPanel};

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

/// Generic state for a compact preferences/settings panel.
///
/// The fixed-size toggle array keeps projection cheap for hot signatures while
/// avoiding product-specific setting names in Radiant-owned APIs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreferencePanelState<const TOGGLES: usize> {
    /// Whether the panel is visible.
    pub visible: bool,
    /// Primary editable text value shown in the panel.
    pub primary_text_value: String,
    /// Enabled states for product-defined toggles.
    pub toggles: [bool; TOGGLES],
    /// Optional auxiliary path, destination, or detail label.
    pub auxiliary_label: Option<String>,
}

impl<const TOGGLES: usize> Default for PreferencePanelState<TOGGLES> {
    fn default() -> Self {
        Self {
            visible: false,
            primary_text_value: String::new(),
            toggles: [false; TOGGLES],
            auxiliary_label: None,
        }
    }
}

impl<const TOGGLES: usize> PreferencePanelState<TOGGLES> {
    /// Build preference panel state from explicit generic fields.
    pub fn new(
        visible: bool,
        primary_text_value: impl Into<String>,
        toggles: [bool; TOGGLES],
        auxiliary_label: Option<String>,
    ) -> Self {
        Self {
            visible,
            primary_text_value: primary_text_value.into(),
            toggles,
            auxiliary_label,
        }
    }

    /// Return one toggle state by stable host-defined index.
    pub fn toggle_enabled(&self, index: usize) -> bool {
        self.toggles.get(index).copied().unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::{OptionItem, PreferencePanelState, SummaryField};

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

    #[test]
    fn preference_panel_state_preserves_visibility_text_toggles_and_auxiliary_label() {
        let panel = PreferencePanelState::new(
            true,
            "Default",
            [true, false, true],
            Some(String::from("Destination")),
        );

        assert!(panel.visible);
        assert_eq!(panel.primary_text_value, "Default");
        assert_eq!(panel.toggles, [true, false, true]);
        assert!(panel.toggle_enabled(0));
        assert!(!panel.toggle_enabled(1));
        assert!(!panel.toggle_enabled(99));
        assert_eq!(panel.auxiliary_label.as_deref(), Some("Destination"));
    }
}
