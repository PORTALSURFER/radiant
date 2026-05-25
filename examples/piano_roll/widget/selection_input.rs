use radiant::prelude::*;
use radiant::widgets::PointerModifiers;

use super::super::{NoteSelectionMode, PianoRollMessage, drag::PianoDrag, widget::PianoRollWidget};

impl PianoRollWidget {
    pub(in crate::piano_roll::widget) fn message_for_finished_drag(
        &self,
        grid: Rect,
        position: Point,
        drag: PianoDrag,
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
        if let PianoDrag::Velocity { ids, velocity } = drag {
            return Some(WidgetOutput::custom(PianoRollMessage::SetVelocity {
                ids,
                velocity,
            }));
        }
        Some(WidgetOutput::custom(drag.message_for(
            grid,
            self.viewport,
            position,
        )))
    }

    fn marquee_selection_output(
        &self,
        grid: Rect,
        position: Point,
        start: Point,
        modifiers: PointerModifiers,
    ) -> WidgetOutput {
        let rect = rect_from_points(start, position).clamp_to(grid);
        WidgetOutput::custom(PianoRollMessage::SelectNotes {
            ids: self.note_ids_intersecting(grid, rect),
            mode: marquee_selection_mode(modifiers),
        })
    }

    fn note_ids_intersecting(&self, grid: Rect, rect: Rect) -> Vec<u32> {
        self.notes
            .iter()
            .filter(|note| rects_overlap(self.note_rect(grid, **note), rect))
            .map(|note| note.id)
            .collect()
    }
}

fn marquee_selection_mode(modifiers: PointerModifiers) -> NoteSelectionMode {
    if modifiers.shift && modifiers.command {
        NoteSelectionMode::Add
    } else {
        NoteSelectionMode::Replace
    }
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
