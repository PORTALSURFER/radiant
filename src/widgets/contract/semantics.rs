//! Optional semantic capability for public widgets.

use super::{
    hit_test::{WidgetHitTest, WidgetHitTestRevision},
    pointer_motion::{WidgetPointerMotion, WidgetPointerMotionRevision},
};
use crate::gui::automation::{AutomationLiveRegion, AutomationNodeSemantics, AutomationRole};
use crate::widgets::{WidgetCommon, contract::FocusBehavior};
use std::{any::Any, collections::BTreeMap, fmt, rc::Rc};

/// The original semantics-only [`WidgetCapabilities`] contract.
pub const WIDGET_CAPABILITIES_V1_CONTRACT_VERSION: u16 = 1;

/// Current [`WidgetCapabilities`] contract with optional behavior descriptors.
///
/// A descriptor with a different revision is treated as having no optional
/// capabilities by the runtime until the descriptor contract is updated.
pub const WIDGET_CAPABILITIES_CONTRACT_VERSION: u16 = 2;

pub(crate) const fn supports_semantics_contract(version: u16) -> bool {
    matches!(
        version,
        WIDGET_CAPABILITIES_V1_CONTRACT_VERSION | WIDGET_CAPABILITIES_CONTRACT_VERSION
    )
}

pub(crate) const fn supports_extended_capabilities(version: u16) -> bool {
    version == WIDGET_CAPABILITIES_CONTRACT_VERSION
}

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

/// Typed revision evidence for one exported [`WidgetSemantics`] capability.
///
/// Exact values are compared by their `Eq` implementations and concrete
/// types. The value is UI-local, so it may retain arbitrary `Rc`-owned state;
/// hashes and caller-provided fingerprints are intentionally not part of this
/// contract. The conservative default is used when a capability cannot prove
/// that its semantic output is unchanged.
#[derive(Clone)]
pub struct WidgetSemanticsRevision {
    representation: SemanticsRevisionRepresentation,
}

#[derive(Clone, Default)]
enum SemanticsRevisionRepresentation {
    #[default]
    Conservative,
    Exact(Rc<dyn SemanticsRevisionValue>),
}

impl Default for WidgetSemanticsRevision {
    fn default() -> Self {
        Self::conservative()
    }
}

impl fmt::Debug for WidgetSemanticsRevision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WidgetSemanticsRevision")
            .field(
                "representation",
                &match self.representation {
                    SemanticsRevisionRepresentation::Conservative => "conservative",
                    SemanticsRevisionRepresentation::Exact(_) => "exact",
                },
            )
            .finish()
    }
}

impl PartialEq for WidgetSemanticsRevision {
    fn eq(&self, other: &Self) -> bool {
        match (&self.representation, &other.representation) {
            (
                SemanticsRevisionRepresentation::Conservative,
                SemanticsRevisionRepresentation::Conservative,
            ) => true,
            (
                SemanticsRevisionRepresentation::Exact(previous),
                SemanticsRevisionRepresentation::Exact(current),
            ) => previous.equals(&**current),
            _ => false,
        }
    }
}

impl Eq for WidgetSemanticsRevision {}

trait SemanticsRevisionValue: Any {
    fn equals(&self, other: &dyn SemanticsRevisionValue) -> bool;
}

impl<T> SemanticsRevisionValue for T
where
    T: Eq + 'static,
{
    fn equals(&self, other: &dyn SemanticsRevisionValue) -> bool {
        other
            .as_any()
            .downcast_ref::<T>()
            .is_some_and(|candidate| self == candidate)
    }
}

impl dyn SemanticsRevisionValue {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl WidgetSemanticsRevision {
    /// Return the safe fallback for semantic capabilities that cannot prove
    /// exact changes.
    #[must_use]
    pub const fn conservative() -> Self {
        Self {
            representation: SemanticsRevisionRepresentation::Conservative,
        }
    }

    /// Build exact typed evidence for the capability's semantic output.
    #[must_use]
    pub fn exact<T>(value: T) -> Self
    where
        T: Eq + 'static,
    {
        Self {
            representation: SemanticsRevisionRepresentation::Exact(Rc::new(value)),
        }
    }

    pub(crate) fn is_exact(&self) -> bool {
        matches!(
            self.representation,
            SemanticsRevisionRepresentation::Exact(_)
        )
    }
}

/// Object-safe semantic capability exported by an interactive widget.
pub trait WidgetSemantics {
    /// Return typed revision evidence for this capability's semantic output.
    ///
    /// Existing implementations inherit the conservative default. A custom
    /// capability may return [`WidgetSemanticsRevision::exact`] when all
    /// semantic output changes are represented by an `Eq + 'static` value.
    fn revision(&self) -> WidgetSemanticsRevision {
        WidgetSemanticsRevision::conservative()
    }

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

    /// Advertise explicit automation action names when role-derived defaults
    /// do not fully describe this widget's interaction policy.
    ///
    /// Advertisement is observational only. Runtime action dispatch still
    /// revalidates the live widget's separate accessibility-action contract.
    fn automation_available_actions(&self) -> Option<Vec<String>> {
        None
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
    /// Optional event-aware hit-test and cursor capability.
    pub hit_test: Option<&'a dyn WidgetHitTest>,
    /// Optional stable pointer-motion and capture-routing capability.
    pub pointer_motion: Option<&'a dyn WidgetPointerMotion>,
}

impl<'a> WidgetCapabilities<'a> {
    /// Build a descriptor with no optional capabilities.
    pub const fn none() -> Self {
        Self {
            contract_version: WIDGET_CAPABILITIES_CONTRACT_VERSION,
            semantics: None,
            hit_test: None,
            pointer_motion: None,
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

    /// Add an event-aware hit-test and cursor capability to this descriptor.
    pub fn hit_test(mut self, hit_test: &'a dyn WidgetHitTest) -> Self {
        self.hit_test = Some(hit_test);
        self
    }

    /// Add a stable pointer-motion capability to this descriptor.
    pub fn pointer_motion(mut self, pointer_motion: &'a dyn WidgetPointerMotion) -> Self {
        self.pointer_motion = Some(pointer_motion);
        self
    }

    /// Return whether this descriptor exports automation semantics.
    pub const fn has_semantics(&self) -> bool {
        supports_semantics_contract(self.contract_version) && self.semantics.is_some()
    }

    /// Return whether this descriptor exports event-aware hit testing.
    pub const fn has_hit_test(&self) -> bool {
        supports_extended_capabilities(self.contract_version) && self.hit_test.is_some()
    }

    /// Return whether this descriptor exports stable pointer-motion behavior.
    pub const fn has_pointer_motion(&self) -> bool {
        supports_extended_capabilities(self.contract_version) && self.pointer_motion.is_some()
    }

    /// Return revision evidence for the optional semantics capability.
    ///
    /// The revision hook is queried without evaluating any semantic-output
    /// methods such as role, label, value, or metadata accessors.
    pub fn semantics_revision(&self) -> Option<WidgetSemanticsRevision> {
        if self.has_semantics() {
            self.semantics.map(WidgetSemantics::revision)
        } else {
            None
        }
    }

    /// Return revision evidence for the optional hit-test capability.
    pub fn hit_test_revision(&self) -> Option<WidgetHitTestRevision> {
        if self.has_hit_test() {
            self.hit_test.map(WidgetHitTest::revision)
        } else {
            None
        }
    }

    /// Return revision evidence for the optional pointer-motion capability.
    pub fn pointer_motion_revision(&self) -> Option<WidgetPointerMotionRevision> {
        if self.has_pointer_motion() {
            self.pointer_motion.map(WidgetPointerMotion::revision)
        } else {
            None
        }
    }
}

pub(crate) fn resolve_automation_semantics(
    common: &WidgetCommon,
    capabilities: WidgetCapabilities<'_>,
) -> AutomationNodeSemantics {
    if capabilities.has_semantics()
        && let Some(semantics) = capabilities.semantics
    {
        return semantics.resolve_automation_semantics(common);
    }
    fallback_automation_semantics(common)
}

pub(crate) fn automation_available_actions(
    capabilities: WidgetCapabilities<'_>,
) -> Option<Vec<String>> {
    if capabilities.has_semantics() {
        capabilities
            .semantics
            .and_then(WidgetSemantics::automation_available_actions)
    } else {
        None
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
            .field("hit_test", &self.hit_test.is_some())
            .field("pointer_motion", &self.pointer_motion.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::WidgetSemanticsRevision;

    #[test]
    fn semantic_revision_is_typed_and_conservative_by_default() {
        assert_eq!(
            WidgetSemanticsRevision::default(),
            WidgetSemanticsRevision::conservative()
        );
        assert_eq!(
            WidgetSemanticsRevision::exact("label"),
            WidgetSemanticsRevision::exact("label")
        );
        assert_ne!(
            WidgetSemanticsRevision::exact("label"),
            WidgetSemanticsRevision::exact("other")
        );
        assert_ne!(
            WidgetSemanticsRevision::exact(1_u32),
            WidgetSemanticsRevision::exact(1_u64)
        );
    }
}
