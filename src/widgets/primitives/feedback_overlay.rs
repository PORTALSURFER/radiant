//! Passive overlay widget for lightweight feedback tints, edge bands, and progress fills.

use crate::gui::feedback::horizontal_progress_fill_rect;
use crate::gui::paint::BorderSides;
use crate::gui::types::{Rect, Rgba8};
use crate::layout::{LayoutOutput, Vector2};
use crate::runtime::{PaintFillRect, PaintPrimitive};
use crate::theme::ThemeTokens;
use crate::widgets::contract::{FocusBehavior, PaintBounds, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{WidgetInput, WidgetOutput};
use crate::widgets::primitives::support::WidgetCommon;

/// Passive overlay widget for status, loading, drag-hover, or validation feedback.
#[derive(Clone, Debug, PartialEq)]
pub struct FeedbackOverlayWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable overlay paint configuration.
    pub props: FeedbackOverlayProps,
}

/// Immutable feedback overlay paint configuration.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FeedbackOverlayProps {
    /// Optional full-bounds background tint.
    pub background: Option<Rgba8>,
    /// Optional progress fill painted over the background.
    pub progress: Option<FeedbackOverlayProgress>,
    /// Optional edge-band accent.
    pub edge: Option<FeedbackOverlayEdge>,
}

/// Determinate progress fill for [`FeedbackOverlayWidget`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FeedbackOverlayProgress {
    /// Filled fraction clamped into `0.0..=1.0` during painting.
    pub fraction: f32,
    /// Progress fill color.
    pub color: Rgba8,
}

/// Edge-band accent for [`FeedbackOverlayWidget`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FeedbackOverlayEdge {
    /// Edge-band color.
    pub color: Rgba8,
    /// Edge-band thickness in logical pixels.
    pub thickness: f32,
    /// Edges to paint.
    pub sides: BorderSides,
}

/// Named construction fields for [`FeedbackOverlayWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct FeedbackOverlayWidgetParts {
    /// Stable widget identity used by layout, events, and paint.
    pub id: WidgetId,
    /// Intrinsic feedback-overlay sizing contract.
    pub sizing: WidgetSizing,
    /// Overlay paint configuration.
    pub props: FeedbackOverlayProps,
}

impl FeedbackOverlayWidget {
    /// Build a feedback overlay from named construction fields.
    pub fn from_parts(parts: FeedbackOverlayWidgetParts) -> Self {
        let mut common = WidgetCommon::new(parts.id, parts.sizing);
        common.focus = FocusBehavior::None;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            props: parts.props,
        }
    }

    /// Build an empty feedback overlay with fixed sizing.
    pub fn new(id: WidgetId, sizing: WidgetSizing) -> Self {
        Self::from_parts(FeedbackOverlayWidgetParts {
            id,
            sizing,
            props: FeedbackOverlayProps::default(),
        })
    }

    /// Build a fill-style feedback overlay with a generated runtime id.
    pub fn fill() -> Self {
        Self::new(0, WidgetSizing::fixed(Vector2::new(1.0, 1.0)))
    }

    /// Paint a full-bounds background tint.
    pub fn with_background(mut self, color: Rgba8) -> Self {
        self.props.background = Some(color);
        self
    }

    /// Paint a determinate progress fill.
    pub fn with_progress(mut self, fraction: f32, color: Rgba8) -> Self {
        self.props.progress = Some(FeedbackOverlayProgress { fraction, color });
        self
    }

    /// Paint edge-band accents.
    pub fn with_edge(mut self, color: Rgba8, thickness: f32, sides: BorderSides) -> Self {
        self.props.edge = Some(FeedbackOverlayEdge {
            color,
            thickness,
            sides,
        });
        self
    }
}

impl Widget for FeedbackOverlayWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn needs_state_synchronization(&self) -> bool {
        false
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        if let Some(color) = self.props.background {
            push_fill(primitives, self.common.id, bounds, color);
        }
        if let Some(progress) = self.props.progress
            && let Some(rect) = horizontal_progress_fill_rect(bounds, progress.fraction)
        {
            push_fill(primitives, self.common.id, rect, progress.color);
        }
        if let Some(edge) = self.props.edge {
            push_edge(primitives, self.common.id, bounds, edge);
        }
    }
}

fn push_edge(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    bounds: Rect,
    edge: FeedbackOverlayEdge,
) {
    if !bounds.has_finite_positive_area() || !edge.thickness.is_finite() || edge.thickness <= 0.0 {
        return;
    }
    let thickness = edge.thickness.min(bounds.height()).min(bounds.width());
    if edge.sides.top {
        push_fill(
            primitives,
            widget_id,
            bounds.top_edge_strip(thickness),
            edge.color,
        );
    }
    if edge.sides.bottom {
        push_fill(
            primitives,
            widget_id,
            bounds.bottom_edge_strip(thickness),
            edge.color,
        );
    }
    if edge.sides.left {
        push_fill(
            primitives,
            widget_id,
            bounds.left_edge_strip(thickness),
            edge.color,
        );
    }
    if edge.sides.right {
        push_fill(
            primitives,
            widget_id,
            bounds.right_edge_strip(thickness),
            edge.color,
        );
    }
}

fn push_fill(primitives: &mut Vec<PaintPrimitive>, widget_id: WidgetId, rect: Rect, color: Rgba8) {
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id,
        rect,
        color,
    }));
}
