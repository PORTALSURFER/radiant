//! Shared widget contracts for the public `radiant::widgets` surface.
//!
//! These types describe what all first-class widgets have in common before the
//! generic runtime/message surface exists. They intentionally define
//! responsibilities and vocabulary rather than locking `radiant` into one
//! retained-tree implementation.

use crate::gui::types::Vector2;
use crate::layout::{LayoutNode, NodeId};

/// Stable widget identifier shared with layout-node identities.
///
/// Widgets currently compose with public containers by projecting themselves to
/// `LayoutNode::Widget` leaves using the same stable id space.
pub type WidgetId = NodeId;

/// Public widget taxonomy for reusable Radiant primitives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WidgetKind {
    /// Non-interactive text or label content.
    Text,
    /// Momentary action surface activated by click, tap, or keyboard.
    Button,
    /// Boolean or multi-state toggle surface.
    Toggle,
    /// Editable single-line or multi-line text field.
    TextInput,
    /// Scroll affordance that exposes viewport position and drag/page actions.
    Scrollbar,
    /// Compact pointer handle for drag/reorder gestures.
    DragHandle,
    /// Focusable row or item primitive for lists, tables, and menus.
    ListItem,
    /// Selectable content surface for cards, rows, tiles, and options.
    Selectable,
    /// Compact label, badge, or pill surface for status and selectable filters.
    Badge,
    /// Non-interactive panel, card, or grouped content surface.
    Card,
    /// Non-interactive raster image surface.
    Image,
    /// Custom paint surface that owns its own rendering and input interpretation.
    Canvas,
}

/// Shared intrinsic sizing contract for a widget.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetSizing {
    /// Smallest usable size after the host applies layout constraints.
    pub min: Vector2,
    /// Preferred size used for intrinsic measurement in unconstrained layouts.
    pub preferred: Vector2,
    /// Optional text baseline measured from the top edge in logical pixels.
    pub baseline: Option<f32>,
}

impl WidgetSizing {
    /// Create a widget sizing contract from minimum and preferred sizes.
    pub fn new(min: Vector2, preferred: Vector2) -> Self {
        Self {
            min,
            preferred: Vector2::new(preferred.x.max(min.x), preferred.y.max(min.y)),
            baseline: None,
        }
    }

    /// Create a fixed intrinsic size with no separate minimum.
    pub fn fixed(size: Vector2) -> Self {
        Self::new(size, size)
    }

    /// Return this widget's current layout leaf projection.
    ///
    /// This keeps the current public composition path explicit: containers own
    /// placement, while widgets contribute intrinsic size hints into layout.
    pub fn layout_node(self, id: WidgetId) -> LayoutNode {
        LayoutNode::widget(id, self.preferred)
    }

    /// Attach a text baseline to the sizing contract.
    pub fn with_baseline(mut self, baseline: f32) -> Self {
        self.baseline = Some(baseline.max(0.0));
        self
    }
}

/// Focus participation contract for a widget.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FocusBehavior {
    /// Widget cannot receive focus.
    None,
    /// Widget can receive pointer focus but is skipped by keyboard traversal.
    Pointer,
    /// Widget participates in deterministic keyboard focus traversal.
    Keyboard,
}

/// Shared visual-state vocabulary for widget styling and behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct WidgetState {
    /// Pointer is currently hovering the widget.
    pub hovered: bool,
    /// Primary action is currently pressed/armed.
    pub pressed: bool,
    /// Widget currently owns keyboard focus.
    pub focused: bool,
    /// Widget is semantically selected.
    pub selected: bool,
    /// Widget is semantically active/on.
    pub active: bool,
    /// Widget rejects interaction but still paints.
    pub disabled: bool,
    /// Widget is read-only but remains visible/focusable.
    pub read_only: bool,
}

/// Shared paint clipping contract for widgets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PaintBounds {
    /// Paint must stay inside the assigned widget rectangle.
    ClipToRect,
    /// Paint may extend beyond the assigned rectangle when the parent allows it.
    AllowOverflow,
}

/// Shared paint responsibilities required from every widget primitive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PaintContract {
    /// Whether paint is clipped to the assigned widget rectangle.
    pub bounds: PaintBounds,
    /// Whether focus state should be expressed visually by the widget itself.
    pub paints_focus: bool,
    /// Whether selection/active state should be expressed visually by the widget.
    pub paints_state_layers: bool,
}

impl Default for PaintContract {
    fn default() -> Self {
        Self {
            bounds: PaintBounds::ClipToRect,
            paints_focus: true,
            paints_state_layers: true,
        }
    }
}

/// Shared semantic message families that widgets may emit.
///
/// Runtime adapters bind these event families to host-defined message payloads
/// without forcing app-specific action enums into the widget surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WidgetMessageKind {
    /// Activation such as button press or list-item invoke.
    Activate,
    /// Boolean or enum-like value change.
    ValueChanged,
    /// Text-edit delta, commit, or submit intent.
    TextEdited,
    /// Scroll thumb drag, track click, or viewport request.
    ScrollRequested,
    /// Pointer drag gesture lifecycle.
    Dragged,
    /// Row/item invocation distinct from selection.
    ItemInvoked,
    /// Custom pointer/keyboard gesture routed into a canvas surface.
    CanvasInput,
}

/// Shared style vocabulary that avoids app-specific shell naming.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WidgetTone {
    /// Default neutral surface or text treatment.
    Neutral,
    /// Stronger emphasis for primary actions or highlighted content.
    Accent,
    /// Positive or successful confirmation state.
    Success,
    /// Cautionary or warning state.
    Warning,
    /// Destructive or dangerous action state.
    Danger,
}

/// Shared prominence vocabulary for widget styling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WidgetProminence {
    /// Low-chrome treatment that lets surrounding content dominate.
    Subtle,
    /// Standard control treatment.
    Normal,
    /// High-emphasis treatment for primary actions or critical affordances.
    Strong,
}

/// Minimal widget style contract independent from application-specific themes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WidgetStyle {
    /// Semantic tone used to resolve colors and accents.
    pub tone: WidgetTone,
    /// Relative visual weight of the widget chrome.
    pub prominence: WidgetProminence,
}

impl Default for WidgetStyle {
    fn default() -> Self {
        Self {
            tone: WidgetTone::Neutral,
            prominence: WidgetProminence::Normal,
        }
    }
}
