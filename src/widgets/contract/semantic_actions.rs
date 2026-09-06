//! Explicit backend-neutral action data and optional widget execution capability.
use crate::widgets::{
    InteractionProvenance, NumericAccessibilityAction, WidgetOutput, WidgetSemanticsRevision,
};

/// One semantic request, independent of physical pointer/keyboard samples.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SemanticAction {
    /// Transfer keyboard focus to the target.
    Focus,
    /// Activate a button-like target.
    Press,
    /// Toggle a boolean target.
    Toggle,
    /// Select an item.
    Select,
    /// Replace an editable text value.
    SetText(String),
    /// Use the existing typed numeric accessibility lifecycle.
    Numeric(NumericAccessibilityAction),
}
impl SemanticAction {
    /// Stable observational action name used by automation snapshots.
    pub fn identifier(&self) -> &'static str {
        use crate::gui::automation::*;
        match self {
            Self::Focus => AUTOMATION_ACTION_FOCUS,
            Self::Press => AUTOMATION_ACTION_PRESS,
            Self::Toggle => AUTOMATION_ACTION_TOGGLE,
            Self::Select => AUTOMATION_ACTION_SELECT,
            Self::SetText(_) => AUTOMATION_ACTION_SET_TEXT,
            Self::Numeric(NumericAccessibilityAction::Increment) => AUTOMATION_ACTION_INCREMENT,
            Self::Numeric(NumericAccessibilityAction::Decrement) => AUTOMATION_ACTION_DECREMENT,
            Self::Numeric(NumericAccessibilityAction::SetValueText(_)) => {
                AUTOMATION_ACTION_SET_TEXT
            }
        }
    }
}

/// Provenance of an explicit semantic request; no synthetic native samples are invented.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticActionSource {
    /// An accessibility adapter requested the action.
    Accessibility,
    /// Application code or automation requested the action.
    Programmatic,
}
impl SemanticActionSource {
    /// Convert to the existing typed widget-output provenance.
    pub const fn provenance(self) -> InteractionProvenance {
        match self {
            Self::Accessibility => InteractionProvenance::Accessibility,
            Self::Programmatic => InteractionProvenance::Programmatic,
        }
    }
}

/// Exact typed revision evidence for semantic execution policy.
pub type WidgetSemanticActionRevision = WidgetSemanticsRevision;

/// Widget-local result after runtime admission; accepted output uses the ordinary mapper.
#[derive(Clone, Debug)]
pub enum WidgetSemanticActionResult {
    /// The current handler cannot execute this action; no mutation occurred.
    Unsupported,
    /// The action was accepted, with zero or one output for the normal reducer path.
    Accepted(Option<WidgetOutput>),
}

/// Optional execution capability, separate from observational semantic metadata.
pub trait WidgetSemanticActions {
    /// Revision of action support and execution policy; conservative by default.
    fn revision(&self) -> WidgetSemanticActionRevision {
        WidgetSemanticActionRevision::conservative()
    }
    /// Read-only action-shape support; transient eligibility is checked separately.
    /// This must not mutate or invoke application callbacks.
    fn supports(&self, action: &SemanticAction) -> bool;
    /// Execute one already-qualified action. Unsupported actions must remain inert.
    fn dispatch(
        &mut self,
        action: SemanticAction,
        source: SemanticActionSource,
    ) -> WidgetSemanticActionResult;
}

/// Mutable companion to the read-only v2 semantic-action descriptor.
/// It is obtained only after runtime authority and ownership checks.
pub struct WidgetActionCapabilities<'a> {
    version: u16,
    semantic_actions: Option<&'a mut dyn WidgetSemanticActions>,
}
impl<'a> WidgetActionCapabilities<'a> {
    /// Empty source-compatible capability set.
    pub const fn none() -> Self {
        Self {
            version: 1,
            semantic_actions: None,
        }
    }
    /// Register a mutable semantic-action handler.
    pub fn with_semantic_actions(mut self, actions: &'a mut dyn WidgetSemanticActions) -> Self {
        self.semantic_actions = Some(actions);
        self
    }
    /// Override the contract version for compatibility testing. Only version 1 is supported.
    pub const fn with_contract_version(mut self, version: u16) -> Self {
        self.version = version;
        self
    }
    /// Obtain the registered handler only for a supported contract version.
    pub fn into_semantic_actions(self) -> Option<&'a mut dyn WidgetSemanticActions> {
        if self.version == 1 {
            self.semantic_actions
        } else {
            None
        }
    }
}
impl Default for WidgetActionCapabilities<'_> {
    fn default() -> Self {
        Self::none()
    }
}
