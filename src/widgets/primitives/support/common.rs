//! Shared descriptor contract for primitive widgets.

use crate::layout::LayoutNode;
use crate::widgets::contract::{FocusBehavior, WidgetId, WidgetSizing, WidgetState, WidgetStyle};

#[cfg(test)]
#[path = "common/tests.rs"]
mod tests;

/// Shared contract carried by every public widget descriptor.
#[derive(Clone, Debug, PartialEq)]
pub struct WidgetCommon {
    /// Stable widget identifier.
    pub id: WidgetId,
    /// Intrinsic sizing contract exposed to layout containers.
    pub sizing: WidgetSizing,
    /// Focus participation contract.
    pub focus: FocusBehavior,
    /// Paint responsibilities for this widget.
    pub paint: crate::widgets::contract::PaintContract,
    /// Shared style vocabulary independent from any app theme.
    pub style: WidgetStyle,
    /// Current interaction and visual state.
    pub state: WidgetState,
}

impl WidgetCommon {
    /// Build a shared widget contract with neutral defaults.
    pub fn new(id: WidgetId, sizing: WidgetSizing) -> Self {
        Self {
            id,
            sizing,
            focus: FocusBehavior::None,
            paint: Default::default(),
            style: WidgetStyle::default(),
            state: WidgetState::default(),
        }
    }

    /// Return this contract with explicit focus participation.
    ///
    /// Pointer and keyboard focus both make a widget eligible for pointer hit
    /// testing. Custom widgets that implement pointer-motion affordances should
    /// use [`Self::with_pointer_focus`] or [`Self::with_keyboard_focus`] instead
    /// of relying only on `Widget::accepts_pointer_move()`.
    pub fn with_focus(mut self, focus: FocusBehavior) -> Self {
        self.focus = focus;
        self
    }

    /// Return this contract as pointer-focusable but skipped by keyboard traversal.
    ///
    /// This is the usual choice for custom canvas/editor widgets that need
    /// hover, drag, cursor, tooltip, or paint-only overlay input but should not
    /// receive keyboard focus.
    pub fn with_pointer_focus(self) -> Self {
        self.with_focus(FocusBehavior::Pointer)
    }

    /// Return this contract as keyboard-focusable.
    ///
    /// Keyboard-focusable widgets also receive pointer hit testing, so editor
    /// surfaces with both pointer and keyboard behavior can use this single
    /// helper.
    pub fn with_keyboard_focus(self) -> Self {
        self.with_focus(FocusBehavior::Keyboard)
    }

    /// Return this contract configured for widgets that draw their own chrome.
    ///
    /// Custom canvas, image, GPU surface, and overlay widgets often still need
    /// Radiant's sizing, focus, hit testing, and style contracts while painting
    /// their own focus and state affordances. This helper disables Radiant's
    /// default focus and state-layer paint responsibilities without changing
    /// clipping, focus participation, or hit-testing behavior.
    pub fn without_default_chrome(mut self) -> Self {
        self.paint.paints_focus = false;
        self.paint.paints_state_layers = false;
        self
    }

    /// Project this widget into the current public layout leaf representation.
    pub fn layout_node(&self) -> LayoutNode {
        self.sizing.layout_node(self.id)
    }
}
