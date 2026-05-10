//! Shared widget contracts for the public `radiant::widgets` surface.
//!
//! These types describe what all first-class widgets have in common before the
//! generic runtime/message surface exists. They intentionally define
//! responsibilities and vocabulary rather than locking `radiant` into one
//! retained-tree implementation.

use super::{
    WidgetCommon, WidgetInput, WidgetOutput,
    primitives::{TextAlign, TextWrap},
};
use crate::{
    gui::types::{Rect, Vector2},
    layout::{LayoutNode, LayoutOutput, NodeId},
    runtime::PaintPrimitive,
    theme::ThemeTokens,
};
use std::any::Any;

/// Stable widget identifier shared with layout-node identities.
///
/// Widgets currently compose with public containers by projecting themselves to
/// `LayoutNode::Widget` leaves using the same stable id space.
pub type WidgetId = NodeId;

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
    /// Whether this widget's own chrome should block hover chrome on parent containers.
    pub suppresses_container_hover: bool,
}

impl Default for PaintContract {
    fn default() -> Self {
        Self {
            bounds: PaintBounds::ClipToRect,
            paints_focus: true,
            paints_state_layers: true,
            suppresses_container_hover: false,
        }
    }
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

/// Clone support for boxed [`Widget`] trait objects.
pub trait WidgetClone {
    /// Clone this widget into an owned trait object.
    fn clone_box(&self) -> Box<dyn Widget>;
}

impl<T> WidgetClone for T
where
    T: Widget + Clone + 'static,
{
    fn clone_box(&self) -> Box<dyn Widget> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn Widget> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Public object-safe contract for user-defined Radiant widgets.
///
/// Built-in primitives and custom widgets implement this same trait and travel
/// through the runtime, input, message, paint, and application-builder paths
/// without adding a new Radiant enum variant.
pub trait Widget: WidgetClone + Send + Sync + Any {
    /// Return the shared identity, sizing, focus, state, and style contract.
    fn common(&self) -> &WidgetCommon;

    /// Return the shared contract mutably for runtime-owned state updates.
    fn common_mut(&mut self) -> &mut WidgetCommon;

    /// Route one backend-neutral input event into this widget.
    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput>;

    /// Reconcile retained widget-local state from the previous projected widget.
    ///
    /// The generic runtime calls this when a host message reprojects the
    /// declarative surface. Built-in and custom widgets can preserve transient
    /// interaction details such as caret, selection, or drag state without
    /// requiring the runtime controller to know concrete widget types.
    fn synchronize_from_previous(&mut self, _previous: &dyn Widget) {}

    /// Return whether this widget accepts text-editing input while focused.
    fn accepts_text_input(&self) -> bool {
        false
    }

    /// Return the selected text for focused text-editing widgets.
    fn selected_text(&self) -> Option<String> {
        None
    }

    /// Apply a declarative text wrapping policy when this widget supports text layout.
    fn set_text_wrap(&mut self, _wrap: TextWrap) -> bool {
        false
    }

    /// Apply a declarative horizontal text alignment policy when this widget supports text layout.
    fn set_text_align(&mut self, _align: TextAlign) -> bool {
        false
    }

    /// Append backend-neutral paint primitives for this widget.
    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
    );
}

impl dyn Widget {
    /// Return this widget as `Any` for compatibility adapters.
    pub fn as_any(&self) -> &dyn Any {
        self
    }

    /// Return this widget mutably as `Any` for compatibility adapters.
    pub fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Default for WidgetStyle {
    fn default() -> Self {
        Self {
            tone: WidgetTone::Neutral,
            prominence: WidgetProminence::Normal,
        }
    }
}
