use radiant::prelude::*;
use radiant::widgets::PaintBounds;

use super::{
    PianoRollTool,
    drag::PianoDrag,
    geometry::{row_height_for, x_for_beat_view, y_for_pitch_view},
    model::{PianoNote, PianoRollViewport},
};

#[path = "widget/input.rs"]
mod input;
#[path = "widget/navigation_input.rs"]
mod navigation_input;
#[path = "widget/pointer_input.rs"]
mod pointer_input;
#[path = "widget/velocity_input.rs"]
mod velocity_input;

#[derive(Clone, Debug)]
pub(crate) struct PianoRollWidget {
    pub(super) common: WidgetCommon,
    pub(super) notes: Vec<PianoNote>,
    pub(super) selected_note: Option<u32>,
    pub(super) selected_notes: Vec<u32>,
    pub(super) selected_pitch: Option<i32>,
    pub(super) playhead_beat: f32,
    pub(super) viewport: PianoRollViewport,
    pub(super) tool: PianoRollTool,
    pub(crate) hover_note: Option<u32>,
    pub(crate) hover_pitch: Option<i32>,
    pub(crate) active_pitch: Option<i32>,
    pub(super) hover_position: Option<Point>,
    pub(super) drag: Option<PianoDrag>,
}

impl PianoRollWidget {
    pub(crate) fn new(
        notes: Vec<PianoNote>,
        selected_note: Option<u32>,
        selected_notes: Vec<u32>,
        selected_pitch: Option<i32>,
        playhead_beat: f32,
        viewport: PianoRollViewport,
        tool: PianoRollTool,
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
            selected_notes,
            selected_pitch,
            playhead_beat,
            viewport,
            tool,
            hover_note: None,
            hover_pitch: None,
            active_pitch: None,
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
            Point::new(bounds.max.x - 14.0, bounds.max.y - 92.0),
        )
    }

    pub(crate) fn velocity_rect(&self, bounds: Rect) -> Rect {
        let editor = self.editor_rect(bounds);
        Rect::from_min_max(
            Point::new(editor.min.x, editor.max.y + 12.0),
            Point::new(editor.max.x, bounds.max.y - 20.0),
        )
    }

    pub(crate) fn note_rect(&self, grid: Rect, note: PianoNote) -> Rect {
        let x0 = x_for_beat_view(grid, self.viewport, note.start_beat);
        let x1 = x_for_beat_view(grid, self.viewport, note.end_beat());
        let y0 = y_for_pitch_view(grid, self.viewport, note.pitch);
        Rect::from_min_max(
            Point::new(x0, y0 + 2.0),
            Point::new(x1, y0 + row_height_for(grid, self.viewport) - 2.0),
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

    pub(crate) fn note_is_selected(&self, id: u32) -> bool {
        self.selected_note == Some(id) || self.selected_notes.binary_search(&id).is_ok()
    }

    pub(crate) fn marquee_rect(&self) -> Option<Rect> {
        let PianoDrag::Marquee { start, current, .. } = self.drag.as_ref()? else {
            return None;
        };
        Some(rect_from_points(*start, *current))
    }

    pub(crate) fn keyboard_pitch_rect(&self, keyboard: Rect, pitch: i32) -> Rect {
        let y = y_for_pitch_view(keyboard, self.viewport, pitch);
        Rect::from_min_max(
            Point::new(keyboard.min.x, y),
            Point::new(keyboard.max.x, y + row_height_for(keyboard, self.viewport)),
        )
    }

    pub(crate) fn drag_preview_note(&self, grid: Rect, position: Point) -> Option<PianoNote> {
        let drag = self.drag.as_ref()?;
        let source = match drag {
            PianoDrag::Pan { .. } | PianoDrag::Marquee { .. } => return None,
            PianoDrag::Velocity { .. } => return None,
            PianoDrag::Create { pitch, start_beat } => PianoNote {
                id: u32::MAX,
                pitch: *pitch,
                start_beat: *start_beat,
                length_beats: 0.25,
                velocity: 0.86,
            },
            PianoDrag::Move { id, .. }
            | PianoDrag::ResizeStart { id, .. }
            | PianoDrag::ResizeEnd { id, .. } => self.note_by_id(*id)?,
        };
        Some(
            drag.clone()
                .preview_note(grid, self.viewport, position, source),
        )
    }

    pub(crate) fn drag_preview_notes(&self, grid: Rect, position: Point) -> Vec<PianoNote> {
        let Some(PianoDrag::Move { ids, .. }) = self.drag.as_ref() else {
            return self
                .drag_preview_note(grid, position)
                .into_iter()
                .collect::<Vec<_>>();
        };
        ids.iter()
            .filter_map(|id| self.note_by_id(*id))
            .map(|note| {
                self.drag.clone().expect("active drag exists").preview_note(
                    grid,
                    self.viewport,
                    position,
                    note,
                )
            })
            .collect()
    }
}

fn rect_from_points(a: Point, b: Point) -> Rect {
    Rect::from_min_max(
        Point::new(a.x.min(b.x), a.y.min(b.y)),
        Point::new(a.x.max(b.x), a.y.max(b.y)),
    )
}
