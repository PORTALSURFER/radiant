//! Serializable GUI automation snapshot types emitted by the native shell/runtime.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Stable semantic identifier for one automation node in the native shell tree.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AutomationNodeId(pub String);

impl AutomationNodeId {
    /// Create a new automation node identifier from an owned string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Semantic role describing how an automation node behaves in the GUI.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationRole {
    /// Synthetic root of the automation snapshot tree.
    Root,
    /// Grouping container such as a panel or composite section.
    Group,
    /// Major panel surface.
    Panel,
    /// Toolbar or action strip.
    Toolbar,
    /// Tab-strip container.
    TabList,
    /// Toggleable tab node.
    Tab,
    /// Clickable button.
    Button,
    /// Search or text-entry field.
    SearchField,
    /// Slider or continuous meter interaction surface.
    Slider,
    /// Row in a list or table.
    Row,
    /// Table or row-hosting list surface.
    Table,
    /// Waveform interaction canvas.
    WaveformRegion,
    /// Map interaction canvas.
    MapCanvas,
    /// Focusable point inside the map canvas.
    MapPoint,
    /// Status/readout region.
    Readout,
    /// Dialog or modal container.
    Dialog,
}

/// Quantized window-space bounds for one automation node.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutomationBounds {
    /// Left edge in logical window coordinates.
    pub x: f32,
    /// Top edge in logical window coordinates.
    pub y: f32,
    /// Width in logical window coordinates.
    pub width: f32,
    /// Height in logical window coordinates.
    pub height: f32,
}

/// One node in the GUI automation tree.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutomationNodeSnapshot {
    /// Stable semantic identifier for this node.
    pub id: AutomationNodeId,
    /// Behavioral role for this node.
    pub role: AutomationRole,
    /// Optional human-readable label shown by the GUI.
    pub label: Option<String>,
    /// Quantized window-space bounds.
    pub bounds: AutomationBounds,
    /// Optional current value or summary text.
    pub value: Option<String>,
    /// Whether the node is currently enabled.
    pub enabled: bool,
    /// Whether the node is currently selected or active.
    pub selected: bool,
    /// Stable action identifiers that this node can trigger.
    pub available_actions: Vec<String>,
    /// Additional deterministic metadata for AI/test consumers.
    pub metadata: BTreeMap<String, String>,
    /// Child nodes in semantic tree order.
    pub children: Vec<AutomationNodeSnapshot>,
}

/// Full deterministic automation snapshot emitted for one GUI frame/state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GuiAutomationSnapshot {
    /// Schema version for forward-compatible artifact readers.
    pub schema_version: u32,
    /// Quantized viewport width for the captured shell layout.
    pub viewport_width: u32,
    /// Quantized viewport height for the captured shell layout.
    pub viewport_height: u32,
    /// Root semantic automation node.
    pub root: AutomationNodeSnapshot,
}
