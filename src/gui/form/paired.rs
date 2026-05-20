use super::{OptionItem, SummaryField};
use crate::gui::feedback::HealthState;

#[cfg(test)]
#[path = "paired/tests.rs"]
mod tests;

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
