use radiant::prelude::*;

use super::super::{
    PianoRollMessage, drag::PianoDrag, geometry::x_for_beat_view, model::PianoNote,
};
use super::PianoRollWidget;

impl PianoRollWidget {
    pub(in crate::piano_roll::widget) fn handle_velocity_press(
        &mut self,
        lane: Rect,
        position: Point,
    ) -> Option<WidgetOutput> {
        let note = self.velocity_note_at(lane, position)?;
        let ids = if self.note_is_selected(note.id) && !self.selected_notes.is_empty() {
            self.selected_notes.clone()
        } else {
            self.selected_note = Some(note.id);
            self.selected_notes = vec![note.id];
            vec![note.id]
        };
        self.hover_note = None;
        self.hover_position = Some(position);
        self.drag = Some(PianoDrag::Velocity {
            ids,
            velocity: velocity_for_y(lane, position.y),
        });
        None
    }

    pub(in crate::piano_roll::widget) fn update_velocity_drag(
        &mut self,
        bounds: Rect,
        position: Point,
    ) -> Option<WidgetOutput> {
        let velocity_lane = self.velocity_rect(bounds);
        let next_velocity = velocity_for_y(velocity_lane, position.y);
        self.hover_position = Some(position);
        self.hover_note = None;
        self.hover_pitch = None;
        let Some(PianoDrag::Velocity { ids, velocity }) = self.drag.as_mut() else {
            return None;
        };
        if (*velocity - next_velocity).abs() < 0.001 {
            return None;
        }
        *velocity = next_velocity;
        Some(WidgetOutput::custom(PianoRollMessage::SetVelocity {
            ids: ids.clone(),
            velocity: next_velocity,
        }))
    }

    pub(crate) fn velocity_preview_stem_rect(&self, lane: Rect, note: PianoNote) -> Rect {
        let velocity = match self.drag.as_ref() {
            Some(PianoDrag::Velocity { ids, velocity }) if ids.contains(&note.id) => *velocity,
            _ => note.velocity,
        };
        let x0 = x_for_beat_view(lane, self.viewport, note.start_beat);
        let y = lane.max.y - lane.height() * velocity.clamp(0.0, 1.0);
        Rect::from_min_max(Point::new(x0 - 1.0, y), Point::new(x0 + 1.0, lane.max.y))
    }

    pub(crate) fn velocity_handle_rect(&self, lane: Rect, note: PianoNote) -> Rect {
        let stem = self.velocity_preview_stem_rect(lane, note);
        Rect::from_min_size(
            Point::new(stem.center().x - 4.0, stem.min.y - 4.0),
            Vector2::new(8.0, 8.0),
        )
    }

    fn velocity_note_at(&self, lane: Rect, position: Point) -> Option<PianoNote> {
        self.notes
            .iter()
            .rev()
            .copied()
            .find(|note| {
                self.note_is_selected(note.id)
                    && self.velocity_column_rect(lane, *note).contains(position)
            })
            .or_else(|| {
                self.notes
                    .iter()
                    .rev()
                    .copied()
                    .find(|note| self.velocity_handle_rect(lane, *note).contains(position))
            })
    }

    fn velocity_column_rect(&self, lane: Rect, note: PianoNote) -> Rect {
        let x0 = x_for_beat_view(lane, self.viewport, note.start_beat);
        let x1 = x_for_beat_view(lane, self.viewport, note.end_beat()).max(x0 + 8.0);
        Rect::from_min_max(Point::new(x0, lane.min.y), Point::new(x1, lane.max.y))
    }
}

fn velocity_for_y(lane: Rect, y: f32) -> f32 {
    ((lane.max.y - y) / lane.height().max(1.0)).clamp(0.0, 1.0)
}
