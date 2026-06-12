use super::{
    label::{DenseRowLabelParts, push_dense_row_label},
    marker::{DenseRowMarkerStyle, push_dense_row_vertical_marker},
    palette::DenseRowPalette,
    state::DenseRowVisualState,
};
use crate::{
    gui::types::{Point, Rect, Rgba8},
    runtime::{PaintPrimitive, push_fill_rect, push_stroke_rect},
    widgets::WidgetId,
};

/// Optional inset outline for dense-row chrome.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DenseRowOutlineStyle {
    /// Inset from row bounds before stroking.
    pub inset: f32,
    /// Stroke color.
    pub color: Rgba8,
    /// Stroke width.
    pub width: f32,
}

impl DenseRowOutlineStyle {
    /// Build outline paint from inset, color, and stroke width.
    pub const fn new(inset: f32, color: Rgba8, width: f32) -> Self {
        Self {
            inset,
            color,
            width,
        }
    }
}

/// Named dense-row chrome paint fields.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DenseRowChromeParts {
    /// Visual state used to resolve the row fill.
    pub state: DenseRowVisualState,
    /// Fill palette for the supplied state.
    pub palette: DenseRowPalette,
    /// Optional leading-edge marker.
    pub leading_marker: Option<DenseRowMarkerStyle>,
    /// Optional trailing-edge marker.
    pub trailing_marker: Option<DenseRowMarkerStyle>,
    /// Optional inset outline.
    pub outline: Option<DenseRowOutlineStyle>,
}

impl DenseRowChromeParts {
    /// Build dense-row chrome paint fields from state and palette.
    pub const fn new(state: DenseRowVisualState, palette: DenseRowPalette) -> Self {
        Self {
            state,
            palette,
            leading_marker: None,
            trailing_marker: None,
            outline: None,
        }
    }

    /// Add a leading-edge marker.
    pub const fn leading_marker(mut self, marker: DenseRowMarkerStyle) -> Self {
        self.leading_marker = Some(marker);
        self
    }

    /// Add a leading-edge marker when `condition` is true.
    pub const fn leading_marker_if(mut self, condition: bool, marker: DenseRowMarkerStyle) -> Self {
        if condition {
            self.leading_marker = Some(marker);
        }
        self
    }

    /// Add a trailing-edge marker.
    pub const fn trailing_marker(mut self, marker: DenseRowMarkerStyle) -> Self {
        self.trailing_marker = Some(marker);
        self
    }

    /// Add a trailing-edge marker when `condition` is true.
    pub const fn trailing_marker_if(
        mut self,
        condition: bool,
        marker: DenseRowMarkerStyle,
    ) -> Self {
        if condition {
            self.trailing_marker = Some(marker);
        }
        self
    }

    /// Add an inset outline.
    pub const fn outline(mut self, outline: DenseRowOutlineStyle) -> Self {
        self.outline = Some(outline);
        self
    }

    /// Add an inset outline when `condition` is true.
    pub const fn outline_if(mut self, condition: bool, outline: DenseRowOutlineStyle) -> Self {
        if condition {
            self.outline = Some(outline);
        }
        self
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

/// Push standard dense-row chrome in paint priority order.
///
/// This helper is intended for custom list and tree rows that own their label
/// or content paint but want Radiant to keep fill, marker, and outline
/// composition consistent without allocating temporary paint descriptions.
/// Returns the number of primitives appended.
pub fn push_dense_row_chrome(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    bounds: Rect,
    parts: DenseRowChromeParts,
) -> usize {
    let initial_len = primitives.len();
    push_dense_row_fill(primitives, widget_id, bounds, parts.state, parts.palette);
    if let Some(marker) = parts.leading_marker {
        push_dense_row_vertical_marker(primitives, widget_id, bounds, marker.parts, marker.color);
    }
    if let Some(marker) = parts.trailing_marker {
        push_dense_row_vertical_marker(primitives, widget_id, bounds, marker.parts, marker.color);
    }
    if let Some(outline) = parts.outline {
        push_dense_row_inset_stroke(
            primitives,
            widget_id,
            bounds,
            outline.inset,
            outline.color,
            outline.width,
        );
    }
    primitives.len() - initial_len
}

/// Push standard dense-row chrome followed by a centered dense-row label.
///
/// This is useful for custom-painted list and tree rows whose visible content is
/// a single label over standard dense-row feedback. The helper preserves
/// Radiant's chrome-before-text paint order and avoids repeating the same widget
/// identity and bounds plumbing at each host row painter.
/// Returns the number of primitives appended.
pub fn push_dense_row_labeled_chrome(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    bounds: Rect,
    chrome: DenseRowChromeParts,
    label: DenseRowLabelParts,
) -> usize {
    let initial_len = primitives.len();
    push_dense_row_chrome(primitives, widget_id, bounds, chrome);
    push_dense_row_label(primitives, widget_id, bounds, label);
    primitives.len() - initial_len
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
