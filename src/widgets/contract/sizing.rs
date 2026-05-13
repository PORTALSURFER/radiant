//! Widget identity and intrinsic sizing contracts.

use crate::{
    gui::types::Vector2,
    layout::{LayoutNode, NodeId},
};

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
