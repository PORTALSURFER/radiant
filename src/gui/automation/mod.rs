//! Serializable GUI automation snapshot primitives.

mod model;
mod selectors;
mod serialization;

#[cfg(test)]
mod test_fixtures;
#[cfg(test)]
mod tests;

pub use model::{
    AUTOMATION_ACTION_FOCUS, AUTOMATION_ACTION_PRESS, AUTOMATION_ACTION_SELECT,
    AUTOMATION_ACTION_SET_TEXT, AUTOMATION_ACTION_SET_VALUE, AUTOMATION_ACTION_TOGGLE,
    AutomationBounds, AutomationFocusHints, AutomationLiveRegion, AutomationNodeId,
    AutomationNodeSemantics, AutomationNodeSnapshot, AutomationPoint, AutomationRole,
    GuiAutomationSnapshot,
};
pub use serialization::{AutomationTarget, GuiAutomationTargetSnapshot};
