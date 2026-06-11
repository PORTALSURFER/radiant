use crate::{
    gui::types::{Point, Rect, Rgba8},
    runtime::PaintPrimitive,
    widgets::WidgetId,
};

use super::super::selection_geometry::CanvasSelectionGeometry;
use super::handles::{
    CanvasSelectionBodyHandleHitTestParts, CanvasSelectionBodyHandleStyle,
    CanvasSelectionEdgeHitTestParts, CanvasSelectionEdgeVisualStyle,
    CanvasSelectionTrailingControlHitTestParts, CanvasSelectionTrailingControlStyle,
};

use crate::gui::visualization::canvas::DragHandleRole;
/// Hit-test policy for one normalized canvas selection affordance group.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CanvasSelectionAffordanceHitTestParts {
    /// Optional body/move handle hit-test policy.
    pub body: Option<CanvasSelectionBodyHandleHitTestParts>,
    /// Optional resize-edge hit-test policy.
    pub edge: Option<CanvasSelectionEdgeHitTestParts>,
    /// Optional trailing-control hit-test policy.
    pub trailing_control: Option<CanvasSelectionTrailingControlHitTestParts>,
}

impl CanvasSelectionAffordanceHitTestParts {
    /// Build empty selection-affordance hit-test parts.
    pub const fn new() -> Self {
        Self {
            body: None,
            edge: None,
            trailing_control: None,
        }
    }

    /// Include the body/move handle in hit testing.
    pub const fn with_body(mut self, body: CanvasSelectionBodyHandleHitTestParts) -> Self {
        self.body = Some(body);
        self
    }

    /// Include resize edges in hit testing.
    pub const fn with_edge(mut self, edge: CanvasSelectionEdgeHitTestParts) -> Self {
        self.edge = Some(edge);
        self
    }

    /// Include the trailing control in hit testing.
    pub const fn with_trailing_control(
        mut self,
        trailing_control: CanvasSelectionTrailingControlHitTestParts,
    ) -> Self {
        self.trailing_control = Some(trailing_control);
        self
    }
}

/// Reusable dimension policy for a normalized canvas selection affordance group.
///
/// Use this when one selection exposes a consistent set of editor affordances
/// such as a body/move handle, resize edges, and a trailing control. The style
/// owns reusable dimensions; callers still provide current edge hit-test bounds
/// and pointer positions because those are canvas-layout dependent.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CanvasSelectionAffordanceStyle {
    /// Optional body/move handle style.
    pub body: Option<CanvasSelectionBodyHandleStyle>,
    /// Optional resize-edge style.
    pub edge: Option<CanvasSelectionEdgeVisualStyle>,
    /// Optional trailing-control style.
    pub trailing_control: Option<CanvasSelectionTrailingControlStyle>,
}

/// Paint colors and edge bounds for a normalized canvas selection affordance group.
///
/// Use this with [`CanvasSelectionAffordanceStyle::push_fills`] when a custom
/// canvas selection paints several standard affordances with one dimension
/// style while the host still owns the active colors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasSelectionAffordancePaintParts {
    /// Bounds containing resize-edge visuals.
    pub edge_bounds: Rect,
    /// Fill color for the optional body/move handle.
    pub body_color: Option<Rgba8>,
    /// Fill color for the optional resize edges.
    pub edge_color: Option<Rgba8>,
    /// Fill color for the optional trailing control.
    pub trailing_control_color: Option<Rgba8>,
}

impl CanvasSelectionAffordancePaintParts {
    /// Build empty affordance paint parts with edge visual bounds.
    pub const fn new(edge_bounds: Rect) -> Self {
        Self {
            edge_bounds,
            body_color: None,
            edge_color: None,
            trailing_control_color: None,
        }
    }

    /// Set the body/move-handle fill color.
    pub const fn body_color(mut self, color: Rgba8) -> Self {
        self.body_color = Some(color);
        self
    }

    /// Set the resize-edge fill color.
    pub const fn edge_color(mut self, color: Rgba8) -> Self {
        self.edge_color = Some(color);
        self
    }

    /// Set the trailing-control fill color.
    pub const fn trailing_control_color(mut self, color: Rgba8) -> Self {
        self.trailing_control_color = Some(color);
        self
    }
}

impl CanvasSelectionAffordanceStyle {
    /// Build an empty selection-affordance style.
    pub const fn new() -> Self {
        Self {
            body: None,
            edge: None,
            trailing_control: None,
        }
    }

    /// Include a body/move handle.
    pub const fn with_body(mut self, body: CanvasSelectionBodyHandleStyle) -> Self {
        self.body = Some(body);
        self
    }

    /// Include resize edges.
    pub const fn with_edge(mut self, edge: CanvasSelectionEdgeVisualStyle) -> Self {
        self.edge = Some(edge);
        self
    }

    /// Include a trailing control.
    pub const fn with_trailing_control(
        mut self,
        trailing_control: CanvasSelectionTrailingControlStyle,
    ) -> Self {
        self.trailing_control = Some(trailing_control);
        self
    }

    /// Build matching hit-test parts for the supplied pointer position.
    ///
    /// `edge_bounds` may be the full canvas or a narrower strip reserved for
    /// edge hit targets.
    pub const fn hit_test_parts(
        self,
        edge_bounds: Rect,
        point: Point,
    ) -> CanvasSelectionAffordanceHitTestParts {
        let mut parts = CanvasSelectionAffordanceHitTestParts::new();
        if let Some(body) = self.body {
            parts = parts.with_body(body.hit_test_parts(point));
        }
        if let Some(edge) = self.edge {
            parts = parts.with_edge(edge.hit_test_parts(edge_bounds, point));
        }
        if let Some(trailing_control) = self.trailing_control {
            parts = parts.with_trailing_control(trailing_control.hit_test_parts(point));
        }
        parts
    }

    /// Return the first styled affordance containing `point`.
    ///
    /// Priority is inherited from [`CanvasSelectionGeometry::affordance_at_point`]:
    /// trailing controls first, then resize edges, then body/move handles.
    pub fn affordance_at_point(
        self,
        geometry: CanvasSelectionGeometry,
        edge_bounds: Rect,
        point: Point,
    ) -> Option<DragHandleRole> {
        geometry.affordance_at_point(self.hit_test_parts(edge_bounds, point))
    }

    /// Push fills for each configured affordance with a matching paint color.
    ///
    /// Returns the number of fill primitives appended. Missing styles or colors
    /// skip that affordance, so callers can keep one style for hit testing and
    /// paint only the affordances that are active in the current view state.
    pub fn push_fills(
        self,
        primitives: &mut Vec<PaintPrimitive>,
        widget_id: WidgetId,
        geometry: CanvasSelectionGeometry,
        parts: CanvasSelectionAffordancePaintParts,
    ) -> usize {
        let mut appended = 0;
        if let (Some(edge), Some(color)) = (self.edge, parts.edge_color) {
            for role in [DragHandleRole::Start, DragHandleRole::End] {
                if geometry.push_edge_visual_fill(
                    primitives,
                    widget_id,
                    edge.paint_parts(parts.edge_bounds, role, color),
                ) {
                    appended += 1;
                }
            }
        }
        if let (Some(body), Some(color)) = (self.body, parts.body_color)
            && geometry.push_body_handle_fill(primitives, widget_id, body.paint_parts(color))
        {
            appended += 1;
        }
        if let (Some(trailing_control), Some(color)) =
            (self.trailing_control, parts.trailing_control_color)
            && geometry.push_trailing_control_fill(
                primitives,
                widget_id,
                trailing_control.paint_parts(color),
            )
        {
            appended += 1;
        }
        appended
    }
}
