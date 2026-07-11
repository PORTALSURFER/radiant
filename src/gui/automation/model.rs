use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Stable action name for moving keyboard or logical focus to a node.
pub const AUTOMATION_ACTION_FOCUS: &str = "focus";
/// Stable action name for pressing a button-like node.
pub const AUTOMATION_ACTION_PRESS: &str = "press";
/// Stable action name for selecting a row, option, or item.
pub const AUTOMATION_ACTION_SELECT: &str = "select";
/// Stable action name for changing text in an editable text node.
pub const AUTOMATION_ACTION_SET_TEXT: &str = "set_text";
/// Stable action name for changing a continuous value.
pub const AUTOMATION_ACTION_SET_VALUE: &str = "set_value";
/// Stable action name for toggling an on/off value.
pub const AUTOMATION_ACTION_TOGGLE: &str = "toggle";

/// Stable semantic identifier for one automation node in a GUI tree.
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
    /// Toggle or switch with a checked state.
    Toggle,
    /// Generic selectable option, row, tile, or item.
    Selectable,
    /// Plain text label.
    Text,
    /// Editable text field.
    TextInput,
    /// Search or text-entry field.
    SearchField,
    /// Slider or continuous meter interaction surface.
    Slider,
    /// Row in a list or table.
    Row,
    /// Table or row-hosting list surface.
    Table,
    /// Generic timeline or signal-canvas interaction region.
    TimelineRegion,
    /// Generic spatial or point-cloud canvas.
    SpatialCanvas,
    /// Focusable point inside a spatial canvas.
    SpatialPoint,
    /// Status/readout region.
    Readout,
    /// Dialog or modal container.
    Dialog,
    /// Custom widget or host-defined semantic surface.
    Custom,
}

/// Live-region announcement policy carried by backend-neutral automation data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationLiveRegion {
    /// No live-region semantics.
    #[default]
    None,
    /// Polite status/update region.
    Polite,
    /// Assertive status/update region.
    Assertive,
}

/// Optional directional focus hints for future adapters and tests.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutomationFocusHints {
    /// Previous logical focus target.
    pub previous: Option<AutomationNodeId>,
    /// Next logical focus target.
    pub next: Option<AutomationNodeId>,
    /// Upward logical focus target.
    pub up: Option<AutomationNodeId>,
    /// Downward logical focus target.
    pub down: Option<AutomationNodeId>,
    /// Left logical focus target.
    pub left: Option<AutomationNodeId>,
    /// Right logical focus target.
    pub right: Option<AutomationNodeId>,
}

/// Backend-neutral semantic metadata attached to an automation node.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutomationNodeSemantics {
    /// Behavioral role for this node.
    pub role: AutomationRole,
    /// Optional human-readable label shown by the GUI.
    pub label: Option<String>,
    /// Optional longer description for inspectors and future adapters.
    pub description: Option<String>,
    /// Optional current value or summary text.
    pub value_text: Option<String>,
    /// Optional checked state for toggles, checkboxes, and switches.
    pub checked: Option<bool>,
    /// Whether the node is currently selected.
    pub selected: bool,
    /// Whether the node rejects interaction while still painting.
    pub disabled: bool,
    /// Whether the node is read-only but visible or focusable.
    pub read_only: bool,
    /// Whether the node can receive focus.
    pub focusable: bool,
    /// Whether the node currently owns focus.
    pub focused: bool,
    /// Optional tab-order index for tests and future adapters.
    pub tab_index: Option<i32>,
    /// Optional directional focus hints.
    pub focus_hints: AutomationFocusHints,
    /// Optional live-region policy.
    pub live_region: AutomationLiveRegion,
    /// Additional deterministic metadata for automation and test consumers.
    pub metadata: BTreeMap<String, String>,
}

impl AutomationNodeSemantics {
    /// Build neutral semantics for the provided role.
    pub fn new(role: AutomationRole) -> Self {
        Self {
            role,
            label: None,
            description: None,
            value_text: None,
            checked: None,
            selected: false,
            disabled: false,
            read_only: false,
            focusable: false,
            focused: false,
            tab_index: None,
            focus_hints: AutomationFocusHints::default(),
            live_region: AutomationLiveRegion::None,
            metadata: BTreeMap::new(),
        }
    }

    /// Return whether the node is enabled.
    pub const fn enabled(&self) -> bool {
        !self.disabled
    }

    /// Return this semantic payload with a label.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Return this semantic payload with value text.
    pub fn with_value_text(mut self, value: impl Into<String>) -> Self {
        self.value_text = Some(value.into());
        self
    }

    /// Return this semantic payload with checked state.
    pub fn with_checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    /// Return conservative default action identifiers implied by this node's
    /// role and interaction state.
    pub fn default_available_actions(&self) -> Vec<String> {
        if self.disabled {
            return Vec::new();
        }

        let mut actions = Vec::new();
        if self.focusable {
            actions.push(AUTOMATION_ACTION_FOCUS.to_owned());
        }

        match self.role {
            AutomationRole::Button | AutomationRole::Tab => {
                actions.push(AUTOMATION_ACTION_PRESS.to_owned());
            }
            AutomationRole::Toggle => {
                actions.push(AUTOMATION_ACTION_TOGGLE.to_owned());
            }
            AutomationRole::Selectable | AutomationRole::Row => {
                actions.push(AUTOMATION_ACTION_SELECT.to_owned());
            }
            AutomationRole::TextInput | AutomationRole::SearchField if !self.read_only => {
                actions.push(AUTOMATION_ACTION_SET_TEXT.to_owned());
            }
            AutomationRole::Slider if !self.read_only => {
                actions.push(AUTOMATION_ACTION_SET_VALUE.to_owned());
            }
            _ => {}
        }

        actions
    }
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

/// Logical window-space point for automation targets.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutomationPoint {
    /// Horizontal coordinate in logical window space.
    pub x: f32,
    /// Vertical coordinate in logical window space.
    pub y: f32,
}

impl AutomationBounds {
    /// Build automation bounds from a runtime layout rectangle.
    pub fn from_rect(rect: crate::gui::types::Rect) -> Self {
        Self {
            x: rect.min.x,
            y: rect.min.y,
            width: rect.width(),
            height: rect.height(),
        }
    }

    /// Return empty bounds for nodes that do not participate in layout.
    pub const fn zero() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }

    /// Return whether these bounds describe an empty or non-hit-testable area.
    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    /// Return the center point of these bounds in logical window space.
    pub fn center(&self) -> AutomationPoint {
        AutomationPoint {
            x: self.x + self.width * 0.5,
            y: self.y + self.height * 0.5,
        }
    }
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
    /// Additional deterministic metadata for automation and test consumers.
    pub metadata: BTreeMap<String, String>,
    /// Rich backend-neutral semantics for tests, devtools, and future adapters.
    pub semantics: AutomationNodeSemantics,
    /// Child nodes in semantic tree order.
    pub children: Vec<AutomationNodeSnapshot>,
}

impl AutomationNodeSnapshot {
    /// Build a snapshot node from bounds and rich semantics.
    pub fn from_semantics(
        id: AutomationNodeId,
        bounds: AutomationBounds,
        semantics: AutomationNodeSemantics,
    ) -> Self {
        Self {
            id,
            role: semantics.role,
            label: semantics.label.clone(),
            bounds,
            value: semantics.value_text.clone(),
            enabled: semantics.enabled(),
            selected: semantics.selected,
            available_actions: semantics.default_available_actions(),
            metadata: semantics.metadata.clone(),
            semantics,
            children: Vec::new(),
        }
    }

    /// Return this snapshot node with semantic children.
    pub fn with_children(mut self, children: Vec<AutomationNodeSnapshot>) -> Self {
        self.children = children;
        self
    }
}

/// Full deterministic automation snapshot emitted for one GUI frame/state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GuiAutomationSnapshot {
    /// Schema version for forward-compatible artifact readers.
    pub schema_version: u32,
    /// Quantized viewport width for the captured layout.
    pub viewport_width: u32,
    /// Quantized viewport height for the captured layout.
    pub viewport_height: u32,
    /// Root semantic automation node.
    pub root: AutomationNodeSnapshot,
}
