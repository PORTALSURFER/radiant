use crate::{
    gui::text_layout::{TextLineInsets, centered_text_baseline, centered_text_line},
    gui::types::{Point, Rect, Rgba8, Vector2},
    runtime::{
        PaintPrimitive, PaintText, PaintTextAlign, PaintTextRun, push_fill_rect, push_stroke_rect,
    },
    widgets::{TextWrap, WidgetId},
};

/// Generic visual state for a dense list or tree row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DenseRowVisualState {
    /// The row is selected by the host application.
    pub selected: bool,
    /// Pointer is hovering the row.
    pub hovered: bool,
    /// Primary pointer activation is pressed or armed.
    pub pressed: bool,
    /// The row is the committed target for an active operation.
    pub active_target: bool,
    /// The row is a valid candidate for an active operation.
    pub candidate: bool,
}

/// Fill colors for generic dense-row state projection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DenseRowPalette {
    /// Fill for the selected state.
    pub selected: Option<Rgba8>,
    /// Fill for pointer hover.
    pub hovered: Option<Rgba8>,
    /// Fill for pointer press.
    pub pressed: Option<Rgba8>,
    /// Fill for a committed operation target.
    pub active_target: Option<Rgba8>,
    /// Fill for a hovered operation candidate.
    pub candidate_hovered: Option<Rgba8>,
}

impl DenseRowPalette {
    /// Build an empty dense-row palette.
    pub const fn new() -> Self {
        Self {
            selected: None,
            hovered: None,
            pressed: None,
            active_target: None,
            candidate_hovered: None,
        }
    }

    /// Set the selected fill color.
    pub const fn selected(mut self, color: Rgba8) -> Self {
        self.selected = Some(color);
        self
    }

    /// Set the hovered fill color.
    pub const fn hovered(mut self, color: Rgba8) -> Self {
        self.hovered = Some(color);
        self
    }

    /// Set the pressed fill color.
    pub const fn pressed(mut self, color: Rgba8) -> Self {
        self.pressed = Some(color);
        self
    }

    /// Set the committed operation-target fill color.
    pub const fn active_target(mut self, color: Rgba8) -> Self {
        self.active_target = Some(color);
        self
    }

    /// Set the hovered operation-candidate fill color.
    pub const fn candidate_hovered(mut self, color: Rgba8) -> Self {
        self.candidate_hovered = Some(color);
        self
    }
}

/// Edge for dense-row marker geometry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DenseRowMarkerEdge {
    /// Marker is inset from the leading edge.
    #[default]
    Leading,
    /// Marker is inset from the trailing edge.
    Trailing,
}

/// Named fields for projecting a vertical row marker.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DenseRowMarkerParts {
    /// Which horizontal edge owns the marker.
    pub edge: DenseRowMarkerEdge,
    /// Marker width in logical pixels.
    pub width: f32,
    /// Inset from the owning horizontal edge.
    pub edge_inset: f32,
    /// Inset applied to top and bottom before centering the marker.
    pub vertical_inset: f32,
    /// Minimum marker height when the row is taller than the inset area.
    pub min_height: f32,
}

impl DenseRowMarkerParts {
    /// Build marker parts for a leading-edge marker.
    pub const fn leading(width: f32) -> Self {
        Self::new(DenseRowMarkerEdge::Leading, width)
    }

    /// Build marker parts for a trailing-edge marker.
    pub const fn trailing(width: f32) -> Self {
        Self::new(DenseRowMarkerEdge::Trailing, width)
    }

    /// Build marker parts for the supplied edge.
    pub const fn new(edge: DenseRowMarkerEdge, width: f32) -> Self {
        Self {
            edge,
            width,
            edge_inset: 1.0,
            vertical_inset: 3.0,
            min_height: 8.0,
        }
    }

    /// Set inset from the owning horizontal edge.
    pub const fn edge_inset(mut self, inset: f32) -> Self {
        self.edge_inset = inset;
        self
    }

    /// Set top and bottom inset before centering the marker.
    pub const fn vertical_inset(mut self, inset: f32) -> Self {
        self.vertical_inset = inset;
        self
    }

    /// Set the minimum marker height.
    pub const fn min_height(mut self, height: f32) -> Self {
        self.min_height = height;
        self
    }
}

/// Named fields for painting a dense-row label.
#[derive(Clone, Debug, PartialEq)]
pub struct DenseRowLabelParts {
    /// Text to paint.
    pub text: PaintText,
    /// Text color.
    pub color: Rgba8,
    /// Horizontal text inset.
    pub inset_x: f32,
    /// Additional vertical offset after centering.
    pub offset_y: f32,
    /// Horizontal alignment inside the label rectangle.
    pub align: PaintTextAlign,
    /// Text wrapping policy.
    pub wrap: TextWrap,
}

impl DenseRowLabelParts {
    /// Build dense-row label parts with left alignment and no wrapping.
    pub fn new(text: impl Into<PaintText>, color: Rgba8) -> Self {
        Self {
            text: text.into(),
            color,
            inset_x: 4.0,
            offset_y: 0.0,
            align: PaintTextAlign::Left,
            wrap: TextWrap::None,
        }
    }

    /// Set horizontal text inset.
    pub fn inset_x(mut self, inset: f32) -> Self {
        self.inset_x = inset.max(0.0);
        self
    }

    /// Set additional vertical text offset after centering.
    pub fn offset_y(mut self, offset: f32) -> Self {
        self.offset_y = offset;
        self
    }

    /// Set horizontal alignment.
    pub fn align(mut self, align: PaintTextAlign) -> Self {
        self.align = align;
        self
    }

    /// Set wrapping policy.
    pub fn wrap(mut self, wrap: TextWrap) -> Self {
        self.wrap = wrap;
        self
    }
}

/// Return a readable label font size for a dense row height.
///
/// Custom tree and list row painters can use this with Radiant text-line
/// helpers to keep label sizing consistent across compact, medium, and taller
/// dense rows.
pub fn dense_row_label_font_size(row_height: f32) -> f32 {
    if row_height >= 38.0 {
        18.0
    } else if row_height >= 28.0 {
        14.0
    } else {
        13.0
    }
}

/// Return the highest-priority fill color for a dense row state.
pub fn dense_row_fill_color(state: DenseRowVisualState, palette: DenseRowPalette) -> Option<Rgba8> {
    if state.active_target {
        palette.active_target
    } else if state.hovered && state.candidate {
        palette.candidate_hovered
    } else if state.pressed {
        palette.pressed
    } else if state.hovered {
        palette.hovered
    } else if state.selected {
        palette.selected
    } else {
        None
    }
}

/// Push the highest-priority dense-row fill for the supplied state and palette.
///
/// Returns `true` when a fill primitive was appended. This is the paint-plan
/// counterpart to [`dense_row_fill_color`] for custom list and tree rows that
/// reuse Radiant's dense-row state priority.
pub fn push_dense_row_fill(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    bounds: Rect,
    state: DenseRowVisualState,
    palette: DenseRowPalette,
) -> bool {
    let Some(color) = dense_row_fill_color(state, palette) else {
        return false;
    };
    if !bounds.has_finite_positive_area() {
        return false;
    }
    push_fill_rect(primitives, widget_id, bounds, color);
    true
}

/// Push a vertically centered dense-row label.
///
/// Returns `true` when a text primitive was appended.
pub fn push_dense_row_label(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    bounds: Rect,
    parts: DenseRowLabelParts,
) -> bool {
    if parts.text.is_empty() || !bounds.has_finite_positive_area() {
        return false;
    }
    let font_size = dense_row_label_font_size(bounds.height());
    let rect = centered_text_line(
        bounds,
        font_size,
        TextLineInsets::horizontal(parts.inset_x),
        parts.offset_y,
    );
    primitives.push(PaintPrimitive::Text(PaintTextRun {
        widget_id,
        text: parts.text,
        rect,
        font_size,
        baseline: centered_text_baseline(rect, font_size),
        color: parts.color,
        align: parts.align,
        wrap: parts.wrap,
    }));
    true
}

/// Project an inset rectangle, returning `None` when the inset collapses it.
pub fn dense_row_inset_rect(bounds: Rect, inset: f32) -> Option<Rect> {
    if !inset.is_finite() || inset < 0.0 {
        return None;
    }
    let rect = Rect::from_min_max(
        Point::new(bounds.min.x + inset, bounds.min.y + inset),
        Point::new(bounds.max.x - inset, bounds.max.y - inset),
    );
    (rect.width() > 0.0 && rect.height() > 0.0).then_some(rect)
}

/// Push an inset dense-row outline when the inset produces visible geometry.
///
/// Returns `true` when a stroke primitive was appended.
pub fn push_dense_row_inset_stroke(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    bounds: Rect,
    inset: f32,
    color: Rgba8,
    width: f32,
) -> bool {
    if width <= 0.0 || !width.is_finite() {
        return false;
    }
    let Some(rect) = dense_row_inset_rect(bounds, inset) else {
        return false;
    };
    push_stroke_rect(primitives, widget_id, rect, color, width);
    true
}

/// Project a vertically centered marker on one edge of a dense row.
pub fn dense_row_vertical_marker_rect(bounds: Rect, parts: DenseRowMarkerParts) -> Option<Rect> {
    if parts.width <= 0.0
        || parts.edge_inset < 0.0
        || parts.vertical_inset < 0.0
        || parts.min_height < 0.0
        || !parts.width.is_finite()
        || !parts.edge_inset.is_finite()
        || !parts.vertical_inset.is_finite()
        || !parts.min_height.is_finite()
        || bounds.width() <= 0.0
        || bounds.height() <= 0.0
    {
        return None;
    }
    let available_height = (bounds.height() - parts.vertical_inset * 2.0).max(0.0);
    let marker_height = available_height.max(parts.min_height).min(bounds.height());
    let x = match parts.edge {
        DenseRowMarkerEdge::Leading => bounds.min.x + parts.edge_inset,
        DenseRowMarkerEdge::Trailing => bounds.max.x - parts.edge_inset - parts.width,
    };
    Some(Rect::from_min_size(
        Point::new(x, bounds.min.y + (bounds.height() - marker_height) * 0.5),
        Vector2::new(parts.width, marker_height),
    ))
}

/// Push a vertically centered marker on one edge of a dense row.
///
/// Returns `true` when a marker primitive was appended.
pub fn push_dense_row_vertical_marker(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    bounds: Rect,
    parts: DenseRowMarkerParts,
    color: Rgba8,
) -> bool {
    let Some(rect) = dense_row_vertical_marker_rect(bounds, parts) else {
        return false;
    };
    push_fill_rect(primitives, widget_id, rect, color);
    true
}
