use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::model::{
    AutomationBounds, AutomationNodeId, AutomationNodeSnapshot, AutomationPoint, AutomationRole,
};

/// Flattened automation target derived from one semantic node.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutomationTarget {
    /// Stable semantic identifier for this target.
    pub id: AutomationNodeId,
    /// Preorder index in the flattened automation tree.
    pub tree_index: usize,
    /// Depth in the semantic automation tree.
    pub depth: usize,
    /// Root-to-target semantic path.
    pub path: Vec<AutomationNodeId>,
    /// Behavioral role for this target.
    pub role: AutomationRole,
    /// Optional human-readable label shown by the GUI.
    pub label: Option<String>,
    /// Optional longer description for inspectors and adapters.
    pub description: Option<String>,
    /// Optional current value or summary text.
    pub value: Option<String>,
    /// Quantized window-space bounds.
    pub bounds: AutomationBounds,
    /// Center point in logical window space, suitable for pointer automation.
    pub center: AutomationPoint,
    /// Whether this target is currently enabled.
    pub enabled: bool,
    /// Whether this target is currently selected or active.
    pub selected: bool,
    /// Optional checked state for toggle-like targets.
    pub checked: Option<bool>,
    /// Whether this target can receive focus.
    pub focusable: bool,
    /// Whether this target currently owns focus.
    pub focused: bool,
    /// Whether this target is a concrete interaction target.
    pub interaction_target: bool,
    /// Stable action identifiers that this target can trigger.
    pub available_actions: Vec<String>,
    /// Additional deterministic metadata for automation and test consumers.
    pub metadata: BTreeMap<String, String>,
}

impl AutomationTarget {
    /// Build a flattened target from a tree node and traversal metadata.
    pub fn from_node(
        node: &AutomationNodeSnapshot,
        depth: usize,
        tree_index: usize,
        path: Vec<AutomationNodeId>,
    ) -> Self {
        let interaction_target =
            node.enabled && !node.bounds.is_empty() && !node.available_actions.is_empty();
        Self {
            id: node.id.clone(),
            tree_index,
            depth,
            path,
            role: node.role,
            label: node.label.clone(),
            description: node.semantics.description.clone(),
            value: node.value.clone(),
            bounds: node.bounds,
            center: node.bounds.center(),
            enabled: node.enabled,
            selected: node.selected,
            checked: node.semantics.checked,
            focusable: node.semantics.focusable,
            focused: node.semantics.focused,
            interaction_target,
            available_actions: node.available_actions.clone(),
            metadata: node.metadata.clone(),
        }
    }

    /// Return the most useful human-facing text for inspectors and scripts.
    pub fn display_text(&self) -> Option<&str> {
        self.label
            .as_deref()
            .or(self.value.as_deref())
            .or(self.description.as_deref())
    }
}

/// Flattened automation target list for one GUI frame/state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GuiAutomationTargetSnapshot {
    /// Schema version for forward-compatible artifact readers.
    pub schema_version: u32,
    /// Quantized viewport width for the captured layout.
    pub viewport_width: u32,
    /// Quantized viewport height for the captured layout.
    pub viewport_height: u32,
    /// Flattened targets in semantic tree preorder.
    pub targets: Vec<AutomationTarget>,
}
