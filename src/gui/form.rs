//! Generic form and picker primitives.

mod numeric;

use crate::gui::feedback::HealthState;

pub use numeric::{
    DecimalTextInputPolicy, parse_finite_decimal_text, rounded_scaled_u16,
    sanitize_decimal_text_insert,
};

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

/// Field currently expanded inside a paired picker surface.
///
/// Paired pickers are useful for option panels that expose the same group/item/
/// numeric controls for two related sides, such as primary/secondary endpoints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PairedPickerTarget {
    /// Group picker for the primary side.
    PrimaryGroup,
    /// Item picker for the primary side.
    PrimaryItem,
    /// Numeric picker for the primary side.
    PrimaryNumber,
    /// Group picker for the secondary side.
    SecondaryGroup,
    /// Item picker for the secondary side.
    SecondaryItem,
    /// Numeric picker for the secondary side.
    SecondaryNumber,
}

/// Raw value carried by one paired-picker option.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PairedPickerValue<Text = String, Number = u32> {
    /// Primary-side group value, or `None` for an inherited/default value.
    PrimaryGroup(Option<Text>),
    /// Primary-side item value, or `None` for an inherited/default value.
    PrimaryItem(Option<Text>),
    /// Primary-side numeric value, or `None` for an inherited/default value.
    PrimaryNumber(Option<Number>),
    /// Secondary-side group value, or `None` for an inherited/default value.
    SecondaryGroup(Option<Text>),
    /// Secondary-side item value, or `None` for an inherited/default value.
    SecondaryItem(Option<Text>),
    /// Secondary-side numeric value, or `None` for an inherited/default value.
    SecondaryNumber(Option<Number>),
}

/// Shared panel state for paired endpoint controls with summaries and picker choices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairedStatusPanel<Value = PairedPickerValue<String, u32>> {
    /// Compact chip health state.
    pub status_state: HealthState,
    /// Compact chip label shown in surrounding chrome.
    pub status_label: String,
    /// Optional detail or error text shown inside the panel overview.
    pub detail_label: Option<String>,
    /// Primary group summary row.
    pub primary_group: SummaryField,
    /// Primary item summary row.
    pub primary_item: SummaryField,
    /// Primary numeric-setting summary row.
    pub primary_number: SummaryField,
    /// Secondary group summary row.
    pub secondary_group: SummaryField,
    /// Secondary item summary row.
    pub secondary_item: SummaryField,
    /// Secondary numeric-setting summary row.
    pub secondary_number: SummaryField,
    /// Currently expanded picker, or `None` for the overview.
    pub active_picker: Option<PairedPickerTarget>,
    /// Primary group choices.
    pub primary_group_options: Vec<OptionItem<Value>>,
    /// Primary item choices.
    pub primary_item_options: Vec<OptionItem<Value>>,
    /// Primary numeric-setting choices.
    pub primary_number_options: Vec<OptionItem<Value>>,
    /// Secondary group choices.
    pub secondary_group_options: Vec<OptionItem<Value>>,
    /// Secondary item choices.
    pub secondary_item_options: Vec<OptionItem<Value>>,
    /// Secondary numeric-setting choices.
    pub secondary_number_options: Vec<OptionItem<Value>>,
}

impl<Value> Default for PairedStatusPanel<Value> {
    fn default() -> Self {
        Self {
            status_state: HealthState::default(),
            status_label: String::new(),
            detail_label: None,
            primary_group: SummaryField::default(),
            primary_item: SummaryField::default(),
            primary_number: SummaryField::default(),
            secondary_group: SummaryField::default(),
            secondary_item: SummaryField::default(),
            secondary_number: SummaryField::default(),
            active_picker: None,
            primary_group_options: Vec::new(),
            primary_item_options: Vec::new(),
            primary_number_options: Vec::new(),
            secondary_group_options: Vec::new(),
            secondary_item_options: Vec::new(),
            secondary_number_options: Vec::new(),
        }
    }
}

impl<Value> PairedStatusPanel<Value> {
    /// Return the compact status chip state.
    pub fn status_state(&self) -> HealthState {
        self.status_state
    }

    /// Return the compact status chip label.
    pub fn status_label(&self) -> &str {
        &self.status_label
    }

    /// Return optional detail text for the overview.
    pub fn detail_label(&self) -> Option<&str> {
        self.detail_label.as_deref()
    }

    /// Return the primary group summary field.
    pub fn primary_group(&self) -> &SummaryField {
        &self.primary_group
    }

    /// Return the primary item summary field.
    pub fn primary_item(&self) -> &SummaryField {
        &self.primary_item
    }

    /// Return the primary numeric-setting summary field.
    pub fn primary_number(&self) -> &SummaryField {
        &self.primary_number
    }

    /// Return the secondary group summary field.
    pub fn secondary_group(&self) -> &SummaryField {
        &self.secondary_group
    }

    /// Return the secondary item summary field.
    pub fn secondary_item(&self) -> &SummaryField {
        &self.secondary_item
    }

    /// Return the secondary numeric-setting summary field.
    pub fn secondary_number(&self) -> &SummaryField {
        &self.secondary_number
    }

    /// Return the currently expanded picker target, if any.
    pub fn active_picker(&self) -> Option<PairedPickerTarget> {
        self.active_picker
    }

    /// Return the option list associated with a paired-picker target.
    pub fn options_for(&self, target: PairedPickerTarget) -> &[OptionItem<Value>] {
        match target {
            PairedPickerTarget::PrimaryGroup => &self.primary_group_options,
            PairedPickerTarget::PrimaryItem => &self.primary_item_options,
            PairedPickerTarget::PrimaryNumber => &self.primary_number_options,
            PairedPickerTarget::SecondaryGroup => &self.secondary_group_options,
            PairedPickerTarget::SecondaryItem => &self.secondary_item_options,
            PairedPickerTarget::SecondaryNumber => &self.secondary_number_options,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        OptionItem, PairedPickerTarget, PairedPickerValue, PairedStatusPanel, PreferencePanelState,
        SummaryField,
    };

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

    #[test]
    fn paired_picker_models_cover_primary_and_secondary_fields() {
        let target = PairedPickerTarget::SecondaryNumber;
        let value: PairedPickerValue<String, u32> = PairedPickerValue::PrimaryNumber(Some(48_000));

        assert_eq!(target, PairedPickerTarget::SecondaryNumber);
        assert_eq!(value, PairedPickerValue::PrimaryNumber(Some(48_000)));
    }

    #[test]
    fn paired_status_panel_returns_options_for_target() {
        let panel = PairedStatusPanel {
            active_picker: Some(PairedPickerTarget::SecondaryNumber),
            secondary_number_options: vec![OptionItem {
                label: String::from("Default"),
                selected: true,
                value: PairedPickerValue::<String, u32>::SecondaryNumber(None),
            }],
            ..PairedStatusPanel::default()
        };

        assert_eq!(
            panel.active_picker(),
            Some(PairedPickerTarget::SecondaryNumber)
        );
        assert_eq!(
            panel.options_for(PairedPickerTarget::SecondaryNumber)[0].label,
            "Default"
        );
    }
}
