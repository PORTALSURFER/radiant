//! Optional semantic capability for public widgets.

use crate::gui::automation::{AutomationLiveRegion, AutomationNodeSemantics, AutomationRole};
use crate::widgets::{WidgetCommon, contract::FocusBehavior};
use std::collections::BTreeMap;

/// Contract revision for [`WidgetCapabilities`].
///
/// A descriptor with a different revision is treated as having no optional
/// capabilities by the runtime until the descriptor contract is updated.
pub const WIDGET_CAPABILITIES_CONTRACT_VERSION: u16 = 1;

/// Preserve the historical neutral semantics for widgets that export no
/// optional semantic capability.
pub(super) fn fallback_automation_semantics(common: &WidgetCommon) -> AutomationNodeSemantics {
    let focusable = common.focus != FocusBehavior::None && !common.state.disabled;
    AutomationNodeSemantics {
        role: AutomationRole::Custom,
        label: None,
        description: None,
        value_text: None,
        checked: None,
        selected: common.state.selected,
        disabled: common.state.disabled,
        read_only: common.state.read_only,
        focusable,
        focused: common.state.focused,
        tab_index: (common.focus == FocusBehavior::Keyboard && !common.state.disabled).then_some(0),
        focus_hints: Default::default(),
        live_region: AutomationLiveRegion::None,
        metadata: BTreeMap::new(),
    }
}

/// Object-safe semantic capability exported by an interactive widget.
pub trait WidgetSemantics: Send + Sync {
    /// Return the default automation role for this widget.
    fn automation_role(&self) -> AutomationRole {
        AutomationRole::Custom
    }

    /// Return the human-readable automation label, if one is known.
    fn automation_label(&self) -> Option<String> {
        None
    }

    /// Return longer automation description text, if one is known.
    fn automation_description(&self) -> Option<String> {
        None
    }

    /// Return current automation value text, if one is known.
    fn automation_value_text(&self) -> Option<String> {
        None
    }

    /// Return checked state for toggle-like widgets.
    fn automation_checked(&self) -> Option<bool> {
        None
    }

    /// Return live-region policy for dynamic status widgets.
    fn automation_live_region(&self) -> AutomationLiveRegion {
        AutomationLiveRegion::None
    }

    /// Return deterministic metadata for automation and inspector consumers.
    fn automation_metadata(&self) -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    /// Resolve backend-neutral automation semantics against shared widget state.
    fn resolve_automation_semantics(&self, common: &WidgetCommon) -> AutomationNodeSemantics {
        let focusable = common.focus != FocusBehavior::None && !common.state.disabled;
        AutomationNodeSemantics {
            role: self.automation_role(),
            label: self.automation_label(),
            description: self.automation_description(),
            value_text: self.automation_value_text(),
            checked: self.automation_checked(),
            selected: common.state.selected,
            disabled: common.state.disabled,
            read_only: common.state.read_only,
            focusable,
            focused: common.state.focused,
            tab_index: (common.focus == FocusBehavior::Keyboard && !common.state.disabled)
                .then_some(0),
            focus_hints: Default::default(),
            live_region: self.automation_live_region(),
            metadata: self.automation_metadata(),
        }
    }
}

/// Compact, borrowed descriptor of optional widget capabilities.
///
/// The descriptor is returned for the duration of one runtime query. It does
/// not allocate or retain references into an erased widget entry.
#[derive(Clone, Copy)]
pub struct WidgetCapabilities<'a> {
    /// Descriptor contract revision understood by the runtime.
    pub contract_version: u16,
    /// Optional automation semantics capability.
    pub semantics: Option<&'a dyn WidgetSemantics>,
}

impl<'a> WidgetCapabilities<'a> {
    /// Build a descriptor with no optional capabilities.
    pub const fn none() -> Self {
        Self {
            contract_version: WIDGET_CAPABILITIES_CONTRACT_VERSION,
            semantics: None,
        }
    }

    /// Build an empty descriptor ready for optional capability registration.
    pub const fn new() -> Self {
        Self::none()
    }

    /// Add an automation semantics capability to this descriptor.
    pub fn semantics(mut self, semantics: &'a dyn WidgetSemantics) -> Self {
        self.semantics = Some(semantics);
        self
    }

    /// Return whether this descriptor exports automation semantics.
    pub const fn has_semantics(&self) -> bool {
        self.semantics.is_some()
    }
}

impl Default for WidgetCapabilities<'_> {
    fn default() -> Self {
        Self::none()
    }
}

impl std::fmt::Debug for WidgetCapabilities<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WidgetCapabilities")
            .field("contract_version", &self.contract_version)
            .field("semantics", &self.semantics.is_some())
            .finish()
    }
}
