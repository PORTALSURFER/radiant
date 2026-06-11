use super::{
    TimelineCoordinateMapper, TimelineEditHandle, TimelineEditHandleGeometry, TimelineEditPreview,
};
use crate::gui::types::Point;

impl TimelineEditPreview {
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

    /// Return the first standard edit handle whose rectangle contains `position`.
    pub fn standard_handle_at(
        self,
        mapper: TimelineCoordinateMapper,
        geometry: TimelineEditHandleGeometry,
        position: Point,
    ) -> Option<TimelineEditHandle> {
        self.handle_at(
            mapper,
            geometry,
            TimelineEditHandle::standard_order(),
            position,
        )
    }
}
