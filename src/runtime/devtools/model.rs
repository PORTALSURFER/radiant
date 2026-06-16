use crate::{
    gui::automation::AutomationNodeSemantics,
    gui::types::Rect,
    layout::{LayoutDiagnosticCode, NodeId},
    runtime::{RuntimeDiagnostics, SurfacePaintStats},
    widgets::{FocusBehavior, WidgetState},
};

/// Runtime-local devtools overlay policy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DevtoolsOverlayOptions {
    /// Whether runtime devtools overlay paint is enabled.
    pub enabled: bool,
}

impl DevtoolsOverlayOptions {
    /// Return disabled devtools overlay options.
    pub const fn disabled() -> Self {
        Self { enabled: false }
    }

    /// Return enabled devtools overlay options.
    pub const fn enabled() -> Self {
        Self { enabled: true }
    }

    /// Return whether the devtools overlay is enabled.
    pub const fn is_enabled(self) -> bool {
        self.enabled
    }
}

/// Backend-neutral snapshot for Radiant devtools and debug inspector UIs.
#[derive(Clone, Debug, PartialEq)]
pub struct DevtoolsSnapshot {
    /// Current logical runtime viewport.
    pub viewport: Rect,
    /// Full projected surface tree with runtime interaction and layout metadata.
    pub root: DevtoolsNodeSnapshot,
    /// Best current inspected node candidate.
    pub selected_node_id: Option<NodeId>,
    /// Aggregate paint primitive counts for the current runtime frame.
    pub paint: SurfacePaintStats,
    /// Generic runtime diagnostics available to debug panels.
    pub diagnostics: RuntimeDiagnostics,
}

/// Flattened inspector projection ready for tree/detail devtools views.
#[derive(Clone, Debug, PartialEq)]
pub struct DevtoolsInspectorProjection {
    /// Visible depth-first tree rows.
    pub tree_rows: Vec<DevtoolsTreeRow>,
    /// Lines describing the selected node.
    pub selected_details: Vec<String>,
    /// Lines describing high-level runtime diagnostics.
    pub runtime_details: Vec<String>,
}

/// One visible node row in a devtools tree projection.
#[derive(Clone, Debug, PartialEq)]
pub struct DevtoolsTreeRow {
    /// Stable node id.
    pub node_id: NodeId,
    /// Depth from the projected root.
    pub depth: usize,
    /// Generic node kind.
    pub kind: DevtoolsNodeKind,
    /// Human-readable row label.
    pub label: String,
    /// Whether this row is the selected-node candidate.
    pub selected: bool,
    /// Current resolved bounds, when available.
    pub bounds: Option<Rect>,
    /// Whether this row represents a focusable widget.
    pub focusable: bool,
    /// Whether this row represents hovered widget state.
    pub hovered: bool,
    /// Whether this row represents pressed widget state.
    pub pressed: bool,
    /// Whether this row represents focused widget state.
    pub focused: bool,
    /// Whether this row represents captured pointer state.
    pub captured: bool,
    /// Whether this row represents disabled widget state.
    pub disabled: bool,
    /// Whether this row represents read-only widget state.
    pub read_only: bool,
}

/// One projected surface node in a [`DevtoolsSnapshot`].
#[derive(Clone, Debug, PartialEq)]
pub struct DevtoolsNodeSnapshot {
    /// Stable layout/widget node id.
    pub node_id: NodeId,
    /// Generic projected node kind.
    pub kind: DevtoolsNodeKind,
    /// Current resolved layout bounds, when the node participates in layout.
    pub bounds: Option<Rect>,
    /// Widget-specific runtime state for widget leaves.
    pub widget: Option<DevtoolsWidgetSnapshot>,
    /// Layout diagnostics emitted for this node.
    pub layout_diagnostics: Vec<DevtoolsLayoutDiagnostic>,
    /// Child nodes in surface tree order.
    pub children: Vec<DevtoolsNodeSnapshot>,
}

/// Generic surface node kind shown by devtools.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DevtoolsNodeKind {
    /// Root scene node.
    Scene,
    /// Layout container node.
    Container,
    /// Widget leaf node.
    Widget,
    /// Non-interactive overlay descriptor.
    Overlay,
    /// Floating child tree.
    FloatingLayer,
}

/// Widget leaf state shown by devtools.
#[derive(Clone, Debug, PartialEq)]
pub struct DevtoolsWidgetSnapshot {
    /// Focus participation contract.
    pub focus: FocusBehavior,
    /// Whether the widget can receive runtime focus.
    pub focusable: bool,
    /// Whether the widget participates in keyboard focus traversal.
    pub keyboard_focusable: bool,
    /// Whether the widget can receive pointer hit testing.
    pub receives_pointer_hit_testing: bool,
    /// Whether the widget accepts wheel input before scroll fallback.
    pub accepts_wheel_input: bool,
    /// Whether the widget accepts stable pointer move routing.
    pub accepts_pointer_move: bool,
    /// Whether this widget is the current pointer-capture target.
    pub captured: bool,
    /// Shared widget interaction and visual state.
    pub state: WidgetState,
    /// Backend-neutral automation semantics for this widget.
    pub semantics: AutomationNodeSemantics,
}

/// One layout diagnostic attached to a devtools node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DevtoolsLayoutDiagnostic {
    /// Stable diagnostic category.
    pub code: LayoutDiagnosticCode,
    /// Human-readable diagnostic message.
    pub message: String,
}
