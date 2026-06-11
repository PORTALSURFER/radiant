use crate::gui::types::{Rect, Rgba8};

use super::group::CanvasSelectionAffordancePaintParts;
/// Paint colors for a normalized canvas selection and its standard affordances.
///
/// Hosts supply the base color that matches their domain, interaction state, or
/// theme. Radiant owns the standard selection-fill, boundary-cursor, body,
/// resize-edge, and trailing-control color derivation so canvas widgets do not
/// need to duplicate affordance paint policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CanvasSelectionPaintStyle {
    /// Base color used for derived selection chrome.
    pub base_color: Rgba8,
    /// Alpha for the selected range fill.
    pub fill_alpha: u8,
    /// Alpha for boundary cursors.
    pub cursor_alpha: u8,
    /// Alpha for the body/move affordance.
    pub body_alpha: u8,
    /// Alpha for resize-edge affordances.
    pub edge_alpha: u8,
    /// Alpha for trailing-control affordances.
    pub trailing_control_alpha: u8,
}

impl CanvasSelectionPaintStyle {
    /// Build a standard canvas selection style from a host-supplied base color.
    pub const fn new(base_color: Rgba8) -> Self {
        Self {
            base_color,
            fill_alpha: 48,
            cursor_alpha: 230,
            body_alpha: 185,
            edge_alpha: 220,
            trailing_control_alpha: 235,
        }
    }

    /// Override the selected range fill alpha.
    pub const fn fill_alpha(mut self, fill_alpha: u8) -> Self {
        self.fill_alpha = fill_alpha;
        self
    }

    /// Override the boundary cursor alpha.
    pub const fn cursor_alpha(mut self, cursor_alpha: u8) -> Self {
        self.cursor_alpha = cursor_alpha;
        self
    }

    /// Override the body/move affordance alpha.
    pub const fn body_alpha(mut self, body_alpha: u8) -> Self {
        self.body_alpha = body_alpha;
        self
    }

    /// Override the resize-edge affordance alpha.
    pub const fn edge_alpha(mut self, edge_alpha: u8) -> Self {
        self.edge_alpha = edge_alpha;
        self
    }

    /// Override the trailing-control affordance alpha.
    pub const fn trailing_control_alpha(mut self, trailing_control_alpha: u8) -> Self {
        self.trailing_control_alpha = trailing_control_alpha;
        self
    }

    /// Return the selected range fill color.
    pub const fn fill_color(self) -> Rgba8 {
        self.base_color.with_alpha(self.fill_alpha)
    }

    /// Return the boundary cursor color.
    pub const fn cursor_color(self) -> Rgba8 {
        self.base_color.with_alpha(self.cursor_alpha)
    }

    /// Return the body/move affordance color.
    pub const fn body_color(self) -> Rgba8 {
        self.base_color.with_alpha(self.body_alpha)
    }

    /// Return the resize-edge affordance color.
    pub const fn edge_color(self) -> Rgba8 {
        self.base_color.with_alpha(self.edge_alpha)
    }

    /// Return the trailing-control affordance color.
    pub const fn trailing_control_color(self) -> Rgba8 {
        self.base_color.with_alpha(self.trailing_control_alpha)
    }

    /// Build affordance paint parts using this style's standard colors.
    pub const fn affordance_paint_parts(
        self,
        edge_bounds: Rect,
    ) -> CanvasSelectionAffordancePaintParts {
        CanvasSelectionAffordancePaintParts::new(edge_bounds)
            .body_color(self.body_color())
            .edge_color(self.edge_color())
            .trailing_control_color(self.trailing_control_color())
    }
}
