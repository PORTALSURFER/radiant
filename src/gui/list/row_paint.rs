use crate::gui::types::{Point, Rect, Rgba8, Vector2};

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
