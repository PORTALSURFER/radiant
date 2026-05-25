use radiant::prelude::*;
use radiant::widgets::PointerModifiers;

use super::super::{
    PianoRollMessage, PianoRollTool,
    drag::PianoDrag,
    geometry::{beat_for_x_view, pitch_for_y_view},
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
        if matches!(
            self.drag,
            Some(PianoDrag::Velocity { .. } | PianoDrag::VelocityRelative { .. })
        ) {
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
            let (source_start_beat, source_end_beat) = self.time_selection?;
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
