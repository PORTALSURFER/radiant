use radiant::prelude::*;
use radiant::widgets::PointerModifiers;

use super::super::{
    PianoRollMessage, geometry::row_height_for, model::PianoRollViewport, widget::PianoRollWidget,
};

impl PianoRollWidget {
    pub(in crate::piano_roll::widget) fn handle_wheel(
        &self,
        grid: Rect,
        delta: Vector2,
        modifiers: PointerModifiers,
    ) -> Option<WidgetOutput> {
        if delta.y.abs() >= delta.x.abs() && delta.y.abs() > f32::EPSILON {
            return Some(WidgetOutput::custom(self.zoom_message(delta.y, modifiers)));
        }
        let beat_delta = delta.x * self.viewport.visible_beats / grid.width().max(1.0);
        Some(WidgetOutput::custom(PianoRollMessage::PanViewport {
            beat_delta,
            pitch_delta: 0,
        }))
    }

    fn zoom_message(&self, vertical_delta: f32, modifiers: PointerModifiers) -> PianoRollMessage {
        let zooming_in = vertical_delta < 0.0;
        let time_factor = if zooming_in { 0.8 } else { 1.25 };
        PianoRollMessage::ZoomViewport {
            time_factor: modifiers
                .alt
                .then(|| {
                    self.viewport
                        .can_zoom_time(time_factor)
                        .then_some(time_factor)
                })
                .flatten(),
            rows_delta: if modifiers.alt {
                0
            } else if zooming_in {
                -2
            } else {
                2
            },
        }
    }

    pub(in crate::piano_roll::widget) fn handle_pan_drag(
        &mut self,
        grid: Rect,
        position: Point,
        start: Point,
        start_viewport: PianoRollViewport,
    ) -> Option<WidgetOutput> {
        self.hover_position = Some(position);
        let beat_delta =
            -(position.x - start.x) * start_viewport.visible_beats / grid.width().max(1.0);
        let pitch_delta =
            ((position.y - start.y) / row_height_for(grid, start_viewport).max(1.0)).round() as i32;
        let target = start_viewport.panned(beat_delta, pitch_delta);
        let (beat_delta, pitch_delta) = self.viewport.pan_delta_to(target);
        if beat_delta.abs() < f32::EPSILON && pitch_delta == 0 {
            return None;
        }
        Some(WidgetOutput::custom(PianoRollMessage::PanViewport {
            beat_delta,
            pitch_delta,
        }))
    }
}
