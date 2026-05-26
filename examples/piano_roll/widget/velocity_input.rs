use radiant::gui::visualization::VerticalValueMarker;
use radiant::prelude::*;
use radiant::widgets::PointerModifiers;

use super::super::{
    NoteSelectionMode, PianoRollMessage, drag::PianoDrag, geometry::timeline_value_marker_layout,
    model::PianoNote,
};
use super::PianoRollWidget;

const LIVE_VELOCITY_UPDATE_SELECTION_LIMIT: usize = 512;
const VELOCITY_STEM_WIDTH: f32 = 2.0;
const VELOCITY_HANDLE_SIZE: f32 = 8.0;

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
        let note_was_selected = self.note_is_selected(note.id);
        let mut ids = if note_was_selected && !self.selected_notes.is_empty() {
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
        let start_pointer_velocity = VerticalValueAxis::new(lane, 0.0, 1.0).value_for_y(position.y);
        self.drag = Some(PianoDrag::Velocity {
            start_velocities: self.start_velocities_for(&ids),
            ids,
            start_pointer_velocity,
            current_pointer_velocity: start_pointer_velocity,
        });
        (!note_was_selected && !modifiers.command)
            .then(|| WidgetOutput::custom(PianoRollMessage::SelectNote(note.id)))
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
        let next_velocity = VerticalValueAxis::new(velocity_lane, 0.0, 1.0).value_for_y(position.y);
        self.hover_position = Some(position);
        self.hover_note = None;
        self.hover_velocity_note = None;
        self.hover_pitch = None;
        let ids = match self.drag.as_mut() {
            Some(PianoDrag::Velocity {
                ids,
                current_pointer_velocity,
                ..
            }) => {
                if (*current_pointer_velocity - next_velocity).abs() < 0.001 {
                    return None;
                }
                *current_pointer_velocity = next_velocity;
                ids.clone()
            }
            Some(PianoDrag::VelocityRelative { ids, current_y, .. }) => {
                if (*current_y - position.y).abs() < 0.5 {
                    return None;
                }
                *current_y = position.y;
                ids.clone()
            }
            _ => return None,
        };
        if ids.len() > LIVE_VELOCITY_UPDATE_SELECTION_LIMIT {
            return None;
        }
        let velocities = self
            .drag
            .as_ref()
            .and_then(PianoDrag::velocity_values)
            .unwrap_or_default();
        Some(WidgetOutput::custom(PianoRollMessage::SetVelocities {
            velocities,
        }))
    }

    pub(in crate::piano_roll::widget) fn start_note_velocity_drag(
        &mut self,
        id: u32,
        position: Point,
        modifiers: PointerModifiers,
    ) -> Option<WidgetOutput> {
        let note_was_selected = self.note_is_selected(id);
        let mut ids = if note_was_selected && !self.selected_notes.is_empty() {
            self.selected_notes.clone()
        } else if modifiers.command {
            let mut ids = self.selected_notes.clone();
            ids.push(id);
            ids
        } else {
            self.selected_note = Some(id);
            self.selected_notes = vec![id];
            vec![id]
        };
        SelectionSet::normalize_vec(&mut ids);
        self.hover_note = Some(id);
        self.hover_note_resize_edge = None;
        self.hover_velocity_note = None;
        self.hover_position = Some(position);
        let start_velocities = self.start_velocities_for(&ids);
        self.drag = Some(PianoDrag::VelocityRelative {
            ids,
            start_y: position.y,
            current_y: position.y,
            start_velocities,
        });
        (!note_was_selected && !modifiers.command)
            .then(|| WidgetOutput::custom(PianoRollMessage::SelectNote(id)))
    }

    pub(crate) fn velocity_preview_stem_rect(&self, lane: Rect, note: PianoNote) -> Rect {
        self.velocity_marker(lane, note)
            .map(|marker| marker.stem)
            .unwrap_or_else(|| lane.empty_at_min())
    }

    pub(crate) fn velocity_handle_rect(&self, lane: Rect, note: PianoNote) -> Rect {
        self.velocity_marker(lane, note)
            .map(|marker| marker.handle)
            .unwrap_or_else(|| lane.empty_at_min())
    }

    fn velocity_marker(&self, lane: Rect, note: PianoNote) -> Option<VerticalValueMarker> {
        let velocity = self
            .drag
            .as_ref()
            .and_then(|drag| drag.velocity_for_note(note.id))
            .unwrap_or(note.velocity);
        timeline_value_marker_layout(
            lane,
            self.viewport,
            VELOCITY_STEM_WIDTH,
            VELOCITY_HANDLE_SIZE,
        )
        .marker_unclamped(note.start_beat, velocity)
    }

    pub(crate) fn velocity_marquee_rect(&self) -> Option<Rect> {
        let PianoDrag::VelocityMarquee { start, current, .. } = self.drag.as_ref()? else {
            return None;
        };
        Some(Rect::from_points(*start, *current))
    }

    pub(crate) fn velocity_marquee_note_ids(&self, lane: Rect, rect: Rect) -> Vec<u32> {
        let rect = rect.clamp_to(lane);
        self.notes
            .iter()
            .filter(|note| self.velocity_handle_rect(lane, **note).intersects(rect))
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
