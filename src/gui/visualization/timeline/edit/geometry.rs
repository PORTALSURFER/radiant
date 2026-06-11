use super::{
    TimelineCoordinateMapper, TimelineEditHandle, TimelineEditPreview, TimelineEditRegion,
};
use crate::gui::types::{Point, Rect};

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

impl TimelineEditHandleGeometry {
    /// Build handle projection geometry for a visible edit selection.
    pub const fn new(bounds: Rect, selection_rect: Rect, handle_size: f32) -> Self {
        Self {
            bounds,
            selection_rect,
            handle_size,
        }
    }

    /// Return the effective handle size after clamping to the surface bounds.
    pub fn clamped_handle_size(self) -> f32 {
        normalized_handle_size(self.bounds, self.handle_size)
    }
}

/// Geometry policy for projecting edit-preview regions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineEditRegionGeometry {
    /// Horizontal and vertical bounds of the timeline or signal surface.
    pub bounds: Rect,
    /// Visible rectangle for the edited selection.
    pub selection_rect: Rect,
}

impl TimelineEditRegionGeometry {
    /// Build region projection geometry for a visible edit selection.
    pub const fn new(bounds: Rect, selection_rect: Rect) -> Self {
        Self {
            bounds,
            selection_rect,
        }
    }
}

impl TimelineEditPreview {
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

    /// Build standard handle geometry for the visible edit selection.
    pub fn handle_geometry(
        self,
        mapper: TimelineCoordinateMapper,
        handle_size: f32,
    ) -> Option<TimelineEditHandleGeometry> {
        let selection_rect = self.selection_rect(mapper)?;
        Some(TimelineEditHandleGeometry::new(
            mapper.rect,
            selection_rect,
            handle_size,
        ))
    }

    /// Build standard region geometry for the visible edit selection.
    pub fn region_geometry(
        self,
        mapper: TimelineCoordinateMapper,
    ) -> Option<TimelineEditRegionGeometry> {
        let selection_rect = self.selection_rect(mapper)?;
        Some(TimelineEditRegionGeometry::new(mapper.rect, selection_rect))
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
        let size = geometry.clamped_handle_size();
        let x = mapper.x_for_micros(micros);
        let horizontal = geometry.bounds.vertical_strip_around_x(x, size);
        let vertical =
            edit_handle_vertical_band(geometry.bounds, geometry.selection_rect, handle, size);
        horizontal.intersection(vertical)
    }

    /// Project a standard edit-preview region into a paint rectangle.
    pub fn region_rect(
        self,
        mapper: TimelineCoordinateMapper,
        geometry: TimelineEditRegionGeometry,
        region: TimelineEditRegion,
    ) -> Option<Rect> {
        let selection = self.selection?;
        match region {
            TimelineEditRegion::LeadingInner => {
                let end = self.leading_end_micros.unwrap_or(selection.start_micros);
                if end <= selection.start_micros {
                    return None;
                }
                let x = visible_x_for_micros(mapper, end)?;
                let right_x = x.clamp(geometry.selection_rect.min.x, geometry.selection_rect.max.x);
                Some(
                    geometry
                        .selection_rect
                        .left_edge_strip(right_x - geometry.selection_rect.min.x),
                )
            }
            TimelineEditRegion::TrailingInner => {
                let start = self.trailing_start_micros.unwrap_or(selection.end_micros);
                if start >= selection.end_micros {
                    return None;
                }
                let x = visible_x_for_micros(mapper, start)?;
                let left_x = x.clamp(geometry.selection_rect.min.x, geometry.selection_rect.max.x);
                Some(
                    geometry
                        .selection_rect
                        .right_edge_strip(geometry.selection_rect.max.x - left_x),
                )
            }
            TimelineEditRegion::LeadingOuter => {
                let start = self.leading_inner_start_micros?;
                if start >= selection.start_micros {
                    return None;
                }
                let x = visible_x_for_micros(mapper, start)?;
                let left_x = x.clamp(geometry.bounds.min.x, geometry.selection_rect.min.x);
                let outer_bounds = Rect::from_min_max(
                    Point::new(geometry.bounds.min.x, geometry.selection_rect.min.y),
                    Point::new(geometry.selection_rect.min.x, geometry.selection_rect.max.y),
                );
                Some(outer_bounds.right_edge_strip(geometry.selection_rect.min.x - left_x))
            }
            TimelineEditRegion::TrailingOuter => {
                let end = self.trailing_inner_end_micros?;
                if end <= selection.end_micros {
                    return None;
                }
                let x = visible_x_for_micros(mapper, end)?;
                let right_x = x.clamp(geometry.selection_rect.max.x, geometry.bounds.max.x);
                let outer_bounds = Rect::from_min_max(
                    Point::new(geometry.selection_rect.max.x, geometry.selection_rect.min.y),
                    Point::new(geometry.bounds.max.x, geometry.selection_rect.max.y),
                );
                Some(outer_bounds.left_edge_strip(right_x - geometry.selection_rect.max.x))
            }
        }
    }

    /// Return visible rectangles for the standard edit-preview regions.
    pub fn standard_region_rects(
        self,
        mapper: TimelineCoordinateMapper,
        geometry: TimelineEditRegionGeometry,
    ) -> impl Iterator<Item = (TimelineEditRegion, Rect)> {
        TimelineEditRegion::standard_order()
            .into_iter()
            .filter_map(move |region| {
                self.region_rect(mapper, geometry, region)
                    .map(|rect| (region, rect))
            })
    }

    /// Return visible rectangles for the standard edit-preview handles.
    pub fn standard_handle_rects(
        self,
        mapper: TimelineCoordinateMapper,
        geometry: TimelineEditHandleGeometry,
    ) -> impl Iterator<Item = (TimelineEditHandle, Rect)> {
        TimelineEditHandle::standard_order()
            .into_iter()
            .filter_map(move |handle| {
                self.handle_rect(mapper, geometry, handle)
                    .map(|rect| (handle, rect))
            })
    }
}

fn visible_x_for_micros(mapper: TimelineCoordinateMapper, micros: u32) -> Option<f32> {
    if micros < mapper.viewport.start_micros || micros > mapper.viewport.end_micros {
        return None;
    }
    Some(mapper.x_for_micros(micros))
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
