use radiant::prelude::*;
use radiant::widgets::PointerModifiers;

use super::super::{
    NoteSelectionMode, PianoRollMessage, PianoRollTool,
    drag::PianoDrag,
    geometry::{beat_for_x_view, pitch_for_y_view, row_height_for},
    model::PianoRollViewport,
    widget::PianoRollWidget,
};

impl PianoRollWidget {
    pub(in crate::piano_roll::widget) fn handle_pointer_move(
        &mut self,
        grid: Rect,
        bounds: Rect,
        position: Point,
    ) -> Option<WidgetOutput> {
        self.common.state.hovered = bounds.contains(position);
        let keyboard = self.keyboard_rect(bounds);
        let velocity = self.velocity_rect(bounds);
        self.hover_pitch = hovered_pitch(self.viewport, keyboard, grid, position);
        if let Some(PianoDrag::Pan {
            start,
            viewport: start_viewport,
        }) = self.drag.as_ref()
        {
            return self.handle_pan_drag(grid, position, *start, *start_viewport);
        }
        if let Some(PianoDrag::Marquee {
            ref mut current, ..
        }) = self.drag
        {
            *current = position;
            self.hover_position = Some(position);
            self.hover_note = None;
            self.hover_note_resize_edge = None;
            self.hover_velocity_note = None;
            self.hover_pitch = hovered_pitch(self.viewport, keyboard, grid, position);
            return None;
        }
        if let Some(PianoDrag::TimeSelection {
            ref mut current, ..
        }) = self.drag
        {
            *current = position;
            self.hover_position = Some(position);
            self.hover_note = None;
            self.hover_note_resize_edge = None;
            self.hover_velocity_note = None;
            self.hover_pitch = hovered_pitch(self.viewport, keyboard, grid, position);
            return None;
        }
        if let Some(PianoDrag::MoveTimeSelection {
            ref mut current, ..
        }) = self.drag
        {
            *current = position;
            self.hover_position = Some(position);
            self.hover_note = None;
            self.hover_note_resize_edge = None;
            self.hover_velocity_note = None;
            self.hover_pitch = None;
            return None;
        }
        if matches!(self.drag, Some(PianoDrag::VelocityMarquee { .. })) {
            return self.update_velocity_marquee_drag(position);
        }
        if matches!(self.drag, Some(PianoDrag::Velocity { .. })) {
            return self.update_velocity_drag(bounds, position);
        }
        self.hover_velocity_note = velocity
            .contains(position)
            .then(|| {
                self.velocity_note_at(velocity, position)
                    .map(|note| note.id)
            })
            .flatten();
        self.hover_position = grid.contains(position).then_some(position);
        if self.drag.is_some() {
            return None;
        }
        self.hover_note_resize_edge = None;
        self.hover_note = if self.hover_velocity_note.is_some() {
            None
        } else if let Some((id, edge)) = self.note_resize_edge_at_position(grid, position) {
            self.hover_note_resize_edge = Some(edge);
            Some(id)
        } else {
            self.note_at_position(grid, position)
        };
        None
    }

    pub(in crate::piano_roll::widget) fn handle_primary_press(
        &mut self,
        grid: Rect,
        position: Point,
        modifiers: PointerModifiers,
    ) -> Option<WidgetOutput> {
        self.pointer_modifiers = modifiers;
        let beat = beat_for_x_view(grid, self.viewport, position.x);
        let pitch = pitch_for_y_view(grid, self.viewport, position.y);
        if modifiers.shift {
            self.hover_position = Some(position);
            self.hover_note = None;
            self.hover_note_resize_edge = None;
            self.hover_velocity_note = None;
            self.hover_pitch = Some(pitch);
            self.drag = Some(PianoDrag::Marquee {
                start: position,
                current: position,
                modifiers,
            });
            return None;
        }
        if self.tool != PianoRollTool::Select && self.time_selection_contains(grid, position) {
            let Some((source_start_beat, source_end_beat)) = self.time_selection else {
                return None;
            };
            self.hover_position = Some(position);
            self.hover_note = None;
            self.hover_note_resize_edge = None;
            self.hover_velocity_note = None;
            self.hover_pitch = None;
            self.drag = Some(PianoDrag::MoveTimeSelection {
                source_start_beat,
                source_end_beat,
                grab_beat: self.beat_for_position(grid, position),
                current: position,
            });
            return None;
        }
        if let Some(id) = self.note_at_position(grid, position) {
            return self.handle_note_press(grid, id, position, modifiers);
        }
        self.hover_position = Some(position);
        self.hover_note = None;
        self.hover_note_resize_edge = None;
        self.hover_velocity_note = None;
        self.hover_pitch = Some(pitch);
        if self.tool == PianoRollTool::Select {
            self.drag = Some(PianoDrag::Marquee {
                start: position,
                current: position,
                modifiers,
            });
            return None;
        }
        let cursor_beat = self.resolve_beat(beat);
        self.edit_cursor_beat = Some(cursor_beat);
        self.time_selection = None;
        self.drag = Some(PianoDrag::TimeSelection {
            start: position,
            current: position,
        });
        Some(WidgetOutput::custom(PianoRollMessage::SetCursor {
            beat: cursor_beat,
        }))
    }

    pub(in crate::piano_roll::widget) fn handle_primary_double_click(
        &mut self,
        grid: Rect,
        position: Point,
    ) -> Option<WidgetOutput> {
        if self.note_at_position(grid, position).is_some() {
            return None;
        }
        let beat = beat_for_x_view(grid, self.viewport, position.x);
        let pitch = pitch_for_y_view(grid, self.viewport, position.y);
        self.hover_position = Some(position);
        self.hover_note = None;
        self.hover_note_resize_edge = None;
        self.hover_velocity_note = None;
        self.hover_pitch = Some(pitch);
        let cursor_beat = self.resolve_beat(beat);
        self.edit_cursor_beat = Some(cursor_beat);
        self.time_selection = None;
        self.drag = Some(PianoDrag::create(pitch, cursor_beat));
        Some(WidgetOutput::custom(PianoRollMessage::SetCursor {
            beat: cursor_beat,
        }))
    }

    pub(in crate::piano_roll::widget) fn handle_keyboard_press(
        &mut self,
        keyboard: Rect,
        position: Point,
    ) -> Option<WidgetOutput> {
        let pitch = pitch_for_y_view(keyboard, self.viewport, position.y);
        self.hover_position = Some(position);
        self.hover_note = None;
        self.hover_note_resize_edge = None;
        self.hover_velocity_note = None;
        self.hover_pitch = Some(pitch);
        self.active_pitch = Some(pitch);
        Some(WidgetOutput::custom(PianoRollMessage::SelectPitch(pitch)))
    }

    pub(in crate::piano_roll::widget) fn finish_drag(
        &mut self,
        grid: Rect,
        bounds: Rect,
        position: Point,
        modifiers: PointerModifiers,
    ) -> Option<WidgetOutput> {
        let drag = self.drag.take();
        self.active_pitch = None;
        self.hover_note = self.note_at_position(grid, position);
        self.hover_note_resize_edge = None;
        self.hover_velocity_note = None;
        let keyboard = self.keyboard_rect(bounds);
        self.hover_pitch = hovered_pitch(self.viewport, keyboard, grid, position);
        drag.and_then(|drag| {
            self.message_for_finished_drag(grid, bounds, position, drag, modifiers)
        })
    }

    fn handle_pan_drag(
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

    fn handle_note_press(
        &mut self,
        grid: Rect,
        id: u32,
        position: Point,
        modifiers: PointerModifiers,
    ) -> Option<WidgetOutput> {
        if modifiers.shift || modifiers.command {
            self.hover_note = Some(id);
            self.hover_note_resize_edge = None;
            self.hover_velocity_note = None;
            self.hover_position = Some(position);
            return Some(WidgetOutput::custom(PianoRollMessage::SelectNotes {
                ids: vec![id],
                mode: selection_mode(modifiers),
            }));
        }
        self.start_note_drag(grid, id, position)
    }

    fn start_note_drag(&mut self, grid: Rect, id: u32, position: Point) -> Option<WidgetOutput> {
        let note = self.note_by_id(id)?;
        let ids = if self.note_is_selected(id) && !self.selected_notes.is_empty() {
            self.selected_notes.clone()
        } else {
            self.selected_note = Some(id);
            self.selected_notes = vec![id];
            vec![id]
        };
        self.hover_note = Some(id);
        self.hover_note_resize_edge = None;
        self.hover_velocity_note = None;
        self.drag = Some(PianoDrag::from_note_hit(
            grid,
            self.viewport,
            note,
            ids.clone(),
            position,
        ));
        if ids.len() == 1 && ids[0] == id {
            Some(WidgetOutput::custom(PianoRollMessage::SelectNote(id)))
        } else {
            None
        }
    }
}

fn hovered_pitch(
    viewport: PianoRollViewport,
    keyboard: Rect,
    grid: Rect,
    position: Point,
) -> Option<i32> {
    if keyboard.contains(position) {
        return Some(pitch_for_y_view(keyboard, viewport, position.y));
    }
    if grid.contains(position) {
        return Some(pitch_for_y_view(grid, viewport, position.y));
    }
    None
}

fn selection_mode(modifiers: PointerModifiers) -> NoteSelectionMode {
    if modifiers.command {
        NoteSelectionMode::Toggle
    } else if modifiers.shift {
        NoteSelectionMode::Add
    } else {
        NoteSelectionMode::Replace
    }
}
