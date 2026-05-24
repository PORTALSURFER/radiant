use radiant::prelude::*;
use radiant::widgets::PaintBounds;

use super::{
    drag::PianoDrag,
    geometry::{row_height, x_for_beat, y_for_pitch},
    model::PianoNote,
};

#[path = "widget/input.rs"]
mod input;

#[derive(Clone, Debug)]
pub(crate) struct PianoRollWidget {
    pub(super) common: WidgetCommon,
    pub(super) notes: Vec<PianoNote>,
    pub(super) selected_note: Option<u32>,
    pub(super) playhead_beat: f32,
    pub(crate) hover_note: Option<u32>,
    pub(super) hover_position: Option<Point>,
    pub(super) drag: Option<PianoDrag>,
}

impl PianoRollWidget {
    pub(crate) fn new(
        notes: Vec<PianoNote>,
        selected_note: Option<u32>,
        playhead_beat: f32,
    ) -> Self {
        let mut common = WidgetCommon::new(
            0,
            WidgetSizing::new(Vector2::new(760.0, 340.0), Vector2::new(1000.0, 390.0)),
        );
        common.focus = FocusBehavior::Pointer;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            notes,
            selected_note,
            playhead_beat,
            hover_note: None,
            hover_position: None,
            drag: None,
        }
    }

    pub(super) fn keyboard_rect(&self, bounds: Rect) -> Rect {
        let editor = self.editor_rect(bounds);
        Rect::from_min_max(
            Point::new(bounds.min.x + 12.0, editor.min.y),
            Point::new(editor.min.x - 1.0, editor.max.y),
        )
    }

    pub(crate) fn editor_rect(&self, bounds: Rect) -> Rect {
        Rect::from_min_max(
            Point::new(bounds.min.x + 76.0, bounds.min.y + 36.0),
            Point::new(bounds.max.x - 14.0, bounds.max.y - 20.0),
        )
    }

    pub(crate) fn note_rect(&self, grid: Rect, note: PianoNote) -> Rect {
        let x0 = x_for_beat(grid, note.start_beat);
        let x1 = x_for_beat(grid, note.end_beat());
        let y0 = y_for_pitch(grid, note.pitch);
        Rect::from_min_max(
            Point::new(x0, y0 + 2.0),
            Point::new(x1, y0 + row_height(grid) - 2.0),
        )
    }

    fn note_at_position(&self, grid: Rect, position: Point) -> Option<u32> {
        self.notes
            .iter()
            .rev()
            .find(|note| self.note_rect(grid, **note).contains(position))
            .map(|note| note.id)
    }

    pub(crate) fn note_by_id(&self, id: u32) -> Option<PianoNote> {
        self.notes.iter().copied().find(|note| note.id == id)
    }
}
