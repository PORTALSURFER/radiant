use radiant::prelude::*;
use radiant::widgets::PointerModifiers;

use super::super::{NoteSelectionMode, PianoRollMessage, drag::PianoDrag, widget::PianoRollWidget};

impl PianoRollWidget {
    pub(in crate::piano_roll::widget) fn handle_note_press(
        &mut self,
        grid: Rect,
        id: u32,
        position: Point,
        modifiers: PointerModifiers,
    ) -> Option<WidgetOutput> {
        if modifiers.alt {
            return self.start_note_velocity_drag(id, position, modifiers);
        }
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

fn selection_mode(modifiers: PointerModifiers) -> NoteSelectionMode {
    if modifiers.command {
        NoteSelectionMode::Toggle
    } else if modifiers.shift {
        NoteSelectionMode::Add
    } else {
        NoteSelectionMode::Replace
    }
}
