use radiant::prelude::*;
use radiant::widgets::PaintBounds;

use super::{
    geometry::{track_layout, x_for_beat},
    model::ArrangementClip,
};

#[path = "widget/input.rs"]
mod input;

#[derive(Clone, Debug)]
pub(crate) struct ArrangementOverviewWidget {
    pub(super) common: WidgetCommon,
    pub(super) clips: Vec<ArrangementClip>,
    pub(super) selected_clip: Option<u32>,
    pub(super) playhead_beat: f32,
    pub(crate) hover_clip: Option<u32>,
    pub(super) hover_position: Option<Point>,
}

impl ArrangementOverviewWidget {
    pub(crate) fn new(
        clips: Vec<ArrangementClip>,
        selected_clip: Option<u32>,
        playhead_beat: f32,
    ) -> Self {
        let mut common = WidgetCommon::new(
            0,
            WidgetSizing::new(Vector2::new(620.0, 320.0), Vector2::new(760.0, 390.0)),
        );
        common.focus = FocusBehavior::Pointer;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            clips,
            selected_clip,
            playhead_beat,
            hover_clip: None,
            hover_position: None,
        }
    }

    pub(crate) fn timeline_rect(&self, bounds: Rect) -> Rect {
        Rect::from_min_max(
            Point::new(bounds.min.x + 72.0, bounds.min.y + 34.0),
            Point::new(bounds.max.x - 18.0, bounds.max.y - 16.0),
        )
    }

    pub(crate) fn clip_rect(&self, timeline: Rect, clip: ArrangementClip) -> Rect {
        let x0 = x_for_beat(timeline, clip.start_beat);
        let x1 = x_for_beat(timeline, clip.end_beat());
        let track_rect = track_layout(timeline).lane_rect(clip.track);
        Rect::from_min_max(
            Point::new(x0, track_rect.min.y + 8.0),
            Point::new(x1, track_rect.max.y - 8.0),
        )
    }

    fn clip_at_position(&self, timeline: Rect, position: Point) -> Option<u32> {
        self.clips
            .iter()
            .rev()
            .find(|clip| self.clip_rect(timeline, **clip).contains(position))
            .map(|clip| clip.id)
    }
}
