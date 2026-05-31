use super::TimelineCoordinateMapper;
use crate::gui::{
    range::NormalizedRange,
    types::{Point, Rect},
};

/// Editable range and fade handles for a normalized timeline or signal view.
///
/// The structure is deliberately host-neutral: it models a selected interval,
/// optional leading/trailing handle positions, and optional curve controls.
/// Hosts decide whether those controls represent animation ramps, trim previews,
/// easing handles, or other domain behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct TimelineEditPreview {
    /// Range currently being edited.
    pub selection: Option<NormalizedRange>,
    /// End position for the leading/top handle in normalized milli-units.
    pub leading_end_milli: Option<u16>,
    /// End position for the leading/top handle in normalized micro-units.
    pub leading_end_micros: Option<u32>,
    /// Start position for the leading/bottom handle in normalized milli-units.
    pub leading_inner_start_milli: Option<u16>,
    /// Start position for the leading/bottom handle in normalized micro-units.
    pub leading_inner_start_micros: Option<u32>,
    /// Leading curve tension in normalized milli-units.
    pub leading_curve_milli: Option<u16>,
    /// Start position for the trailing/top handle in normalized milli-units.
    pub trailing_start_milli: Option<u16>,
    /// Start position for the trailing/top handle in normalized micro-units.
    pub trailing_start_micros: Option<u32>,
    /// End position for the trailing/bottom handle in normalized milli-units.
    pub trailing_inner_end_milli: Option<u16>,
    /// End position for the trailing/bottom handle in normalized micro-units.
    pub trailing_inner_end_micros: Option<u32>,
    /// Trailing curve tension in normalized milli-units.
    pub trailing_curve_milli: Option<u16>,
}

/// Standard edit-preview handles for a normalized timeline or signal surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimelineEditHandle {
    /// Leading/top handle at the leading ramp end.
    LeadingEnd,
    /// Leading/bottom handle at the selected range start.
    LeadingStart,
    /// Leading outer handle before the selected range.
    LeadingOuterStart,
    /// Trailing/top handle at the trailing ramp start.
    TrailingStart,
    /// Trailing/bottom handle at the selected range end.
    TrailingEnd,
    /// Trailing outer handle after the selected range.
    TrailingOuterEnd,
}

/// Geometry policy for projecting edit-preview handles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineEditHandleGeometry {
    /// Horizontal and vertical bounds of the timeline or signal surface.
    pub bounds: Rect,
    /// Visible rectangle for the edited selection.
    pub selection_rect: Rect,
    /// Logical handle size in pixels.
    pub handle_size: f32,
}

/// Named edit-preview parts for timeline handle projection.
///
/// Hosts can fill only the handles they need while keeping range, leading
/// handles, trailing handles, and curve controls readable at call sites.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct TimelineEditPreviewParts {
    /// Range currently being edited.
    pub selection: Option<NormalizedRange>,
    /// End position for the leading/top handle in normalized milli-units.
    pub leading_end_milli: Option<u16>,
    /// End position for the leading/top handle in normalized micro-units.
    pub leading_end_micros: Option<u32>,
    /// Start position for the leading/bottom handle in normalized milli-units.
    pub leading_inner_start_milli: Option<u16>,
    /// Start position for the leading/bottom handle in normalized micro-units.
    pub leading_inner_start_micros: Option<u32>,
    /// Leading curve tension in normalized milli-units.
    pub leading_curve_milli: Option<u16>,
    /// Start position for the trailing/top handle in normalized milli-units.
    pub trailing_start_milli: Option<u16>,
    /// Start position for the trailing/top handle in normalized micro-units.
    pub trailing_start_micros: Option<u32>,
    /// End position for the trailing/bottom handle in normalized milli-units.
    pub trailing_inner_end_milli: Option<u16>,
    /// End position for the trailing/bottom handle in normalized micro-units.
    pub trailing_inner_end_micros: Option<u32>,
    /// Trailing curve tension in normalized milli-units.
    pub trailing_curve_milli: Option<u16>,
}

impl TimelineEditPreview {
    /// Build an edit preview from named handle parts.
    pub fn from_parts(parts: TimelineEditPreviewParts) -> Self {
        Self {
            selection: parts.selection,
            leading_end_milli: parts.leading_end_milli,
            leading_end_micros: parts.leading_end_micros,
            leading_inner_start_milli: parts.leading_inner_start_milli,
            leading_inner_start_micros: parts.leading_inner_start_micros,
            leading_curve_milli: parts.leading_curve_milli,
            trailing_start_milli: parts.trailing_start_milli,
            trailing_start_micros: parts.trailing_start_micros,
            trailing_inner_end_milli: parts.trailing_inner_end_milli,
            trailing_inner_end_micros: parts.trailing_inner_end_micros,
            trailing_curve_milli: parts.trailing_curve_milli,
        }
    }

    /// Return the normalized micro-position for a standard edit handle.
    pub fn handle_micros(self, handle: TimelineEditHandle) -> Option<u32> {
        let selection = self.selection?;
        match handle {
            TimelineEditHandle::LeadingEnd => {
                Some(self.leading_end_micros.unwrap_or(selection.start_micros))
            }
            TimelineEditHandle::LeadingStart => {
                self.leading_end_micros.map(|_| selection.start_micros)
            }
            TimelineEditHandle::LeadingOuterStart => self.leading_end_micros.and(
                self.leading_inner_start_micros
                    .or(Some(selection.start_micros)),
            ),
            TimelineEditHandle::TrailingStart => {
                Some(self.trailing_start_micros.unwrap_or(selection.end_micros))
            }
            TimelineEditHandle::TrailingEnd => {
                self.trailing_start_micros.map(|_| selection.end_micros)
            }
            TimelineEditHandle::TrailingOuterEnd => self.trailing_start_micros.and(
                self.trailing_inner_end_micros
                    .or(Some(selection.end_micros)),
            ),
        }
    }

    /// Project the currently visible edit selection into the mapper rectangle.
    pub fn selection_rect(self, mapper: TimelineCoordinateMapper) -> Option<Rect> {
        let selection = self.selection?;
        let viewport = mapper.viewport;
        if selection.end_micros < viewport.start_micros
            || selection.start_micros > viewport.end_micros
        {
            return None;
        }
        let start_x = mapper.x_for_micros(selection.start_micros);
        let end_x = mapper.x_for_micros(selection.end_micros);
        if (end_x - start_x).abs() <= f32::EPSILON {
            return None;
        }
        Some(Rect::from_min_max(
            Point::new(start_x.min(end_x), mapper.rect.min.y),
            Point::new(start_x.max(end_x), mapper.rect.max.y),
        ))
    }

    /// Project a standard edit handle into a hit-test or paint rectangle.
    pub fn handle_rect(
        self,
        mapper: TimelineCoordinateMapper,
        geometry: TimelineEditHandleGeometry,
        handle: TimelineEditHandle,
    ) -> Option<Rect> {
        let micros = self.handle_micros(handle)?;
        if micros < mapper.viewport.start_micros || micros > mapper.viewport.end_micros {
            return None;
        }
        let size = normalized_handle_size(geometry.bounds, geometry.handle_size);
        let x = mapper.x_for_micros(micros);
        let horizontal = geometry.bounds.vertical_strip_around_x(x, size);
        let vertical =
            edit_handle_vertical_band(geometry.bounds, geometry.selection_rect, handle, size);
        horizontal.intersection(vertical)
    }

    /// Return the first standard edit handle whose rectangle contains `position`.
    pub fn handle_at(
        self,
        mapper: TimelineCoordinateMapper,
        geometry: TimelineEditHandleGeometry,
        handles: impl IntoIterator<Item = TimelineEditHandle>,
        position: Point,
    ) -> Option<TimelineEditHandle> {
        handles.into_iter().find(|handle| {
            self.handle_rect(mapper, geometry, *handle)
                .is_some_and(|rect| rect.contains(position))
        })
    }
}

fn normalized_handle_size(bounds: Rect, handle_size: f32) -> f32 {
    handle_size
        .max(0.0)
        .min(bounds.width().max(1.0))
        .min(bounds.height().max(1.0))
}

fn edit_handle_vertical_band(
    bounds: Rect,
    selection_rect: Rect,
    handle: TimelineEditHandle,
    size: f32,
) -> Rect {
    match handle {
        TimelineEditHandle::LeadingEnd | TimelineEditHandle::TrailingStart => {
            selection_rect.top_edge_strip(size)
        }
        TimelineEditHandle::LeadingStart | TimelineEditHandle::TrailingEnd => {
            selection_rect.bottom_edge_strip(size)
        }
        TimelineEditHandle::LeadingOuterStart | TimelineEditHandle::TrailingOuterEnd => {
            bounds.horizontal_center_strip(size)
        }
    }
}
