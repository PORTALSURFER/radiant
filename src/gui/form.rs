//! Generic form and picker primitives.

mod numeric;
mod paired;

#[cfg(test)]
#[path = "form/tests.rs"]
mod tests;

pub use numeric::{
    DecimalTextInputPolicy, parse_finite_decimal_text, rounded_scaled_u16,
    sanitize_decimal_text_insert,
};
pub use paired::{
    PairedPickerOptions, PairedPickerTarget, PairedPickerValue, PairedStatusHeader,
    PairedStatusPanel, PairedStatusSummaries,
};

/// Named selection state for generic option rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OptionSelectionState {
    /// The option is not selected.
    #[default]
    Unselected,
    /// The option is selected.
    Selected,
}

impl OptionSelectionState {
    /// Build selection state from compatibility flags.
    pub const fn from_selected(selected: bool) -> Self {
        match selected {
            true => Self::Selected,
            false => Self::Unselected,
        }
    }

    const fn is_selected(self) -> bool {
        matches!(self, Self::Selected)
    }
}

/// Explicit parts used to build one generic selectable option.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptionItemParts<Value> {
    /// Human-readable option label.
    pub label: String,
    /// Current option selection state.
    pub selection: OptionSelectionState,
    /// Value applied when the option is chosen.
    pub value: Value,
}

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

impl<Value> OptionItem<Value> {
    /// Build an option item from named generic parts.
    pub fn from_parts(parts: OptionItemParts<Value>) -> Self {
        Self {
            label: parts.label,
            selected: parts.selection.is_selected(),
            value: parts.value,
        }
    }
}

/// Overview row for a labeled field and its current value summary.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SummaryField {
    /// Static row label.
    pub label: String,
    /// Current value summary.
    pub value_label: String,
}

/// Named visibility state for generic preference panel projections.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PreferencePanelVisibility {
    /// The panel is hidden.
    #[default]
    Hidden,
    /// The panel is visible.
    Visible,
}

impl PreferencePanelVisibility {
    /// Build visibility state from compatibility flags.
    pub const fn from_visible(visible: bool) -> Self {
        match visible {
            true => Self::Visible,
            false => Self::Hidden,
        }
    }

    const fn is_visible(self) -> bool {
        matches!(self, Self::Visible)
    }
}

/// Explicit parts used to build generic preference panel state.
///
/// Keeping this as a named projection object makes app-facing call sites
/// readable without introducing product-specific setting names into Radiant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreferencePanelParts<const TOGGLES: usize> {
    /// Whether the panel is visible.
    pub visibility: PreferencePanelVisibility,
    /// Primary editable text value shown in the panel.
    pub primary_text_value: String,
    /// Enabled states for product-defined toggles.
    pub toggles: [bool; TOGGLES],
    /// Optional auxiliary path, destination, or detail label.
    pub auxiliary_label: Option<String>,
}

impl<const TOGGLES: usize> Default for PreferencePanelParts<TOGGLES> {
    fn default() -> Self {
        Self {
            visibility: PreferencePanelVisibility::Hidden,
            primary_text_value: String::new(),
            toggles: [false; TOGGLES],
            auxiliary_label: None,
        }
    }
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
        Self::from_parts(PreferencePanelParts::default())
    }
}

impl<const TOGGLES: usize> PreferencePanelState<TOGGLES> {
    /// Build preference panel state from named generic projection parts.
    pub fn from_parts(parts: PreferencePanelParts<TOGGLES>) -> Self {
        Self {
            visible: parts.visibility.is_visible(),
            primary_text_value: parts.primary_text_value,
            toggles: parts.toggles,
            auxiliary_label: parts.auxiliary_label,
        }
    }

    /// Build preference panel state from explicit generic fields.
    pub fn new(
        visible: bool,
        primary_text_value: impl Into<String>,
        toggles: [bool; TOGGLES],
        auxiliary_label: Option<String>,
    ) -> Self {
        Self::from_parts(PreferencePanelParts {
            visibility: PreferencePanelVisibility::from_visible(visible),
            primary_text_value: primary_text_value.into(),
            toggles,
            auxiliary_label,
        })
    }

    /// Return one toggle state by stable host-defined index.
    pub fn toggle_enabled(&self, index: usize) -> bool {
        self.toggles.get(index).copied().unwrap_or(false)
    }
}
