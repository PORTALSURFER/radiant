use radiant::prelude::*;
use radiant::widgets::PointerModifiers;

use super::super::{
    NoteSelectionMode, drag::PianoDrag, geometry::x_for_beat_view, model::PianoNote,
};
use super::PianoRollWidget;

impl PianoRollWidget {
    pub(in crate::piano_roll::widget) fn handle_velocity_press(
        &mut self,
        lane: Rect,
        position: Point,
        modifiers: PointerModifiers,
    ) -> Option<WidgetOutput> {
        self.pointer_modifiers = modifiers;
        let Some(note) = self.velocity_note_at(lane, position) else {
            return self.start_velocity_marquee(position, modifiers);
        };
        if modifiers.shift {
            return self.start_velocity_marquee(position, modifiers);
        }
        let mut ids = if self.note_is_selected(note.id) && !self.selected_notes.is_empty() {
            self.selected_notes.clone()
        } else {
            self.selected_note = Some(note.id);
            self.selected_notes = vec![note.id];
            vec![note.id]
        };
        SelectionSet::normalize_vec(&mut ids);
        self.hover_note = None;
        self.hover_velocity_note = None;
        self.hover_position = Some(position);
        let start_pointer_velocity = velocity_for_y(lane, position.y);
        self.drag = Some(PianoDrag::Velocity {
            start_velocities: self.start_velocities_for(&ids),
            ids,
            start_pointer_velocity,
            current_pointer_velocity: start_pointer_velocity,
        });
        None
    }

    pub(in crate::piano_roll::widget) fn update_velocity_marquee_drag(
        &mut self,
        position: Point,
    ) -> Option<WidgetOutput> {
        let Some(PianoDrag::VelocityMarquee { current, .. }) = self.drag.as_mut() else {
            return None;
        };
        *current = position;
        self.hover_position = Some(position);
        self.hover_note = None;
        self.hover_velocity_note = None;
        self.hover_pitch = None;
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
        self.hover_velocity_note = None;
        self.hover_pitch = None;
        let Some(PianoDrag::Velocity {
            current_pointer_velocity,
            ..
        }) = self.drag.as_mut()
        else {
            return None;
        };
        if (*current_pointer_velocity - next_velocity).abs() < 0.001 {
            return None;
        }
        *current_pointer_velocity = next_velocity;
        None
    }

    pub(crate) fn velocity_preview_stem_rect(&self, lane: Rect, note: PianoNote) -> Rect {
        let velocity = self
            .drag
            .as_ref()
            .and_then(|drag| drag.velocity_for_note(note.id))
            .unwrap_or(note.velocity);
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

    pub(crate) fn velocity_marquee_rect(&self) -> Option<Rect> {
        let PianoDrag::VelocityMarquee { start, current, .. } = self.drag.as_ref()? else {
            return None;
        };
        Some(rect_from_points(*start, *current))
    }

    pub(crate) fn velocity_marquee_note_ids(&self, lane: Rect, rect: Rect) -> Vec<u32> {
        let rect = rect.clamp_to(lane);
        self.notes
            .iter()
            .filter(|note| rects_overlap(self.velocity_handle_rect(lane, **note), rect))
            .map(|note| note.id)
            .collect()
    }

    pub(crate) fn velocity_marquee_selection_mode(
        &self,
        modifiers: PointerModifiers,
    ) -> NoteSelectionMode {
        if modifiers.shift && modifiers.command {
            NoteSelectionMode::Add
        } else {
            NoteSelectionMode::Replace
        }
    }

    pub(in crate::piano_roll::widget) fn velocity_note_at(
        &self,
        lane: Rect,
        position: Point,
    ) -> Option<PianoNote> {
        self.notes
            .iter()
            .rev()
            .copied()
            .find(|note| self.velocity_handle_rect(lane, *note).contains(position))
    }

    fn start_velocity_marquee(
        &mut self,
        position: Point,
        modifiers: PointerModifiers,
    ) -> Option<WidgetOutput> {
        self.hover_note = None;
        self.hover_velocity_note = None;
        self.hover_pitch = None;
        self.hover_position = Some(position);
        self.drag = Some(PianoDrag::VelocityMarquee {
            start: position,
            current: position,
            modifiers,
        });
        None
    }

    fn start_velocities_for(&self, ids: &[u32]) -> Vec<(u32, f32)> {
        let mut ids = ids.to_vec();
        SelectionSet::normalize_vec(&mut ids);
        let mut velocities = self
            .notes
            .iter()
            .filter(|note| SelectionSet::slice_contains(&ids, &note.id))
            .map(|note| (note.id, note.velocity))
            .collect::<Vec<_>>();
        velocities.sort_unstable_by_key(|(id, _)| *id);
        velocities
    }
}

fn velocity_for_y(lane: Rect, y: f32) -> f32 {
    ((lane.max.y - y) / lane.height().max(1.0)).clamp(0.0, 1.0)
}

fn rect_from_points(a: Point, b: Point) -> Rect {
    Rect::from_min_max(
        Point::new(a.x.min(b.x), a.y.min(b.y)),
        Point::new(a.x.max(b.x), a.y.max(b.y)),
    )
}

fn rects_overlap(a: Rect, b: Rect) -> bool {
    a.min.x <= b.max.x && a.max.x >= b.min.x && a.min.y <= b.max.y && a.max.y >= b.min.y
}
