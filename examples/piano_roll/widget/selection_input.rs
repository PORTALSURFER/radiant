use radiant::prelude::*;
use radiant::widgets::PointerModifiers;

use super::super::{NoteSelectionMode, PianoRollMessage, drag::PianoDrag, widget::PianoRollWidget};

impl PianoRollWidget {
    pub(in crate::piano_roll::widget) fn message_for_finished_drag(
        &self,
        grid: Rect,
        bounds: Rect,
        position: Point,
        drag: PianoDrag,
        modifiers: PointerModifiers,
    ) -> Option<WidgetOutput> {
        if matches!(drag, PianoDrag::Pan { .. }) {
            return None;
        }
        if let PianoDrag::Marquee {
            start, modifiers, ..
        } = drag
        {
            return Some(self.marquee_selection_output(grid, position, start, modifiers));
        }
        if let PianoDrag::VelocityMarquee {
            start, modifiers, ..
        } = drag
        {
            return Some(
                self.velocity_marquee_selection_output(bounds, position, start, modifiers),
            );
        }
        if let Some(velocities) = drag.velocity_values() {
            return Some(WidgetOutput::custom(PianoRollMessage::SetVelocities {
                velocities,
            }));
        }
        if let PianoDrag::TimeSelection { .. } = drag {
            return Some(WidgetOutput::custom(drag.message_for(
                grid,
                self.viewport,
                position,
                self.snap_enabled,
            )));
        }
        if let PianoDrag::MoveTimeSelection {
            source_start_beat,
            source_end_beat,
            grab_beat,
            current,
        } = drag
        {
            let (start_beat, _) = self.moved_time_selection_beats(
                source_start_beat,
                source_end_beat,
                grab_beat,
                current,
                grid,
            );
            if modifiers.command {
                return Some(WidgetOutput::custom(PianoRollMessage::CopyTimeSelection {
                    source_start_beat,
                    source_end_beat,
                    target_start_beat: start_beat,
                }));
            }
            return Some(WidgetOutput::custom(PianoRollMessage::MoveTimeSelection {
                source_start_beat,
                source_end_beat,
                target_start_beat: start_beat,
            }));
        }
        Some(WidgetOutput::custom(drag.message_for(
            grid,
            self.viewport,
            position,
            self.snap_enabled,
        )))
    }

    fn marquee_selection_output(
        &self,
        grid: Rect,
        position: Point,
        start: Point,
        modifiers: PointerModifiers,
    ) -> WidgetOutput {
        let rect = Rect::from_points(start, position).clamp_to(grid);
        WidgetOutput::custom(PianoRollMessage::SelectNotes {
            ids: self.note_ids_intersecting(grid, rect),
            mode: marquee_selection_mode(modifiers),
        })
    }

    fn note_ids_intersecting(&self, grid: Rect, rect: Rect) -> Vec<u32> {
        self.notes
            .iter()
            .filter(|note| self.note_rect(grid, **note).intersects(rect))
            .map(|note| note.id)
            .collect()
    }

    fn velocity_marquee_selection_output(
        &self,
        bounds: Rect,
        position: Point,
        start: Point,
        modifiers: PointerModifiers,
    ) -> WidgetOutput {
        let lane = self.velocity_rect(bounds);
        let rect = Rect::from_points(start, position).clamp_to(lane);
        WidgetOutput::custom(PianoRollMessage::SelectNotes {
            ids: self.velocity_marquee_note_ids(lane, rect),
            mode: self.velocity_marquee_selection_mode(modifiers),
        })
    }
}

fn marquee_selection_mode(modifiers: PointerModifiers) -> NoteSelectionMode {
    if modifiers.shift && modifiers.command {
        NoteSelectionMode::Add
    } else {
        NoteSelectionMode::Replace
    }
}
