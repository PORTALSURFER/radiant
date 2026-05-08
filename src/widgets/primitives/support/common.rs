//! Shared descriptor contract for primitive widgets.

use crate::layout::LayoutNode;
use crate::widgets::contract::{FocusBehavior, WidgetId, WidgetSizing, WidgetState, WidgetStyle};

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

    /// Project this widget into the current public layout leaf representation.
    pub fn layout_node(&self) -> LayoutNode {
        self.sizing.layout_node(self.id)
    }
}
