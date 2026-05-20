use super::super::{ArrangementTimelineWidget, RESIZE_HANDLE_WIDTH, ResizeEdge, TimelineGeometry};
use crate::model::TimelineClip;
use radiant::gui::visualization::{DragHandle, DragHandleRole, drag_handle_at_point};
use radiant::layout::{Point, Rect};

#[derive(Clone, Copy)]
pub(super) struct TimelineClipHandle {
    pub(super) clip_id: u32,
    pub(super) clip_name: &'static str,
    pub(super) clip_lane: usize,
    pub(super) clip_start: u32,
    pub(super) clip_end: u32,
    pub(super) duration: u32,
    role: DragHandleRole,
}

impl TimelineClipHandle {
    pub(super) fn resize_edge(self) -> Option<ResizeEdge> {
        match self.role {
            DragHandleRole::Start => Some(ResizeEdge::Start),
            DragHandleRole::End => Some(ResizeEdge::End),
            _ => None,
        }
    }
}

pub(super) fn clip_handle_at(
    widget: &ArrangementTimelineWidget,
    geometry: TimelineGeometry,
    position: Point,
) -> Option<TimelineClipHandle> {
    widget.clips.iter().rev().find_map(|clip| {
        let role = drag_handle_at_point(&clip_drag_handles(geometry, clip), position)?.role;
        Some(TimelineClipHandle {
            clip_id: clip.id,
            clip_name: clip.name,
            clip_lane: clip.lane,
            clip_start: clip.range.start,
            clip_end: clip.range.end,
            duration: clip.range.duration(),
            role,
        })
    })
}

fn clip_drag_handles(geometry: TimelineGeometry, clip: &TimelineClip) -> [DragHandle; 3] {
    let rect = geometry.clip_rect(clip).inset_vertical(-4.0, -4.0);
    let width = RESIZE_HANDLE_WIDTH.min((rect.width() * 0.5).max(0.0));
    [
        DragHandle::new(DragHandleRole::Body, rect, clip.id as u64),
        DragHandle::new(
            DragHandleRole::Start,
            Rect::from_min_max(rect.min, Point::new(rect.min.x + width, rect.max.y)),
            clip.id as u64,
        ),
        DragHandle::new(
            DragHandleRole::End,
            Rect::from_min_max(Point::new(rect.max.x - width, rect.min.y), rect.max),
            clip.id as u64,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TimelineEditorState;
    use radiant::layout::Vector2;

    #[test]
    fn clip_drag_handles_prioritize_edges_over_body() {
        let widget = ArrangementTimelineWidget::new(&TimelineEditorState::default());
        let geometry = widget.geometry(Rect::from_min_size(
            Point::new(0.0, 0.0),
            Vector2::new(860.0, 252.0),
        ));
        let clip = &widget.clips[0];
        let rect = geometry.clip_rect(clip);

        assert_eq!(
            clip_handle_at(
                &widget,
                geometry,
                Point::new(rect.min.x + 1.0, rect.center().y),
            )
            .map(TimelineClipHandle::resize_edge),
            Some(Some(ResizeEdge::Start))
        );
        assert_eq!(
            clip_handle_at(&widget, geometry, rect.center()).map(TimelineClipHandle::resize_edge),
            Some(None)
        );
        assert_eq!(
            clip_handle_at(
                &widget,
                geometry,
                Point::new(rect.max.x - 1.0, rect.center().y),
            )
            .map(TimelineClipHandle::resize_edge),
            Some(Some(ResizeEdge::End))
        );
    }
}
