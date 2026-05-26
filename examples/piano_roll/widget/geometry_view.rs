use super::super::{
    TOTAL_BEATS,
    geometry::{
        beat_for_x_view, beat_range_rect_view, pitch_layout, quantize_beat, x_for_beat_view,
    },
    model::PianoNote,
};
use super::{NOTE_RESIZE_EDGE_WIDTH, NoteResizeEdge, PianoDrag, PianoRollWidget};
use radiant::gui::visualization::{
    DragHandleRole, drag_handle_at_point, horizontal_resize_edge_handles,
};
use radiant::prelude::*;

impl PianoRollWidget {
    pub(crate) fn keyboard_rect(&self, bounds: Rect) -> Rect {
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
        let row = pitch_layout(grid, self.viewport)
            .pitch_rect(note.pitch)
            .inset_vertical(2.0, 2.0);
        Rect::from_min_max(Point::new(x0, row.min.y), Point::new(x1, row.max.y))
    }

    pub(super) fn note_at_position(&self, grid: Rect, position: Point) -> Option<u32> {
        self.notes
            .iter()
            .rev()
            .find(|note| self.note_rect(grid, **note).contains(position))
            .map(|note| note.id)
    }

    pub(crate) fn note_resize_edge_at_position(
        &self,
        grid: Rect,
        position: Point,
    ) -> Option<(u32, NoteResizeEdge)> {
        self.notes.iter().rev().find_map(|note| {
            let rect = self.note_rect(grid, *note);
            let role = drag_handle_at_point(
                &horizontal_resize_edge_handles(rect, NOTE_RESIZE_EDGE_WIDTH, note.id as u64)?,
                position,
            )?
            .role;
            match role {
                DragHandleRole::Start => Some((note.id, NoteResizeEdge::Start)),
                DragHandleRole::End => Some((note.id, NoteResizeEdge::End)),
                _ => None,
            }
        })
    }

    pub(crate) fn note_by_id(&self, id: u32) -> Option<PianoNote> {
        self.notes.iter().copied().find(|note| note.id == id)
    }

    pub(crate) fn note_is_selected(&self, id: u32) -> bool {
        self.selected_note == Some(id) || SelectionSet::slice_contains(&self.selected_notes, &id)
    }

    pub(crate) fn selected_note_count(&self) -> usize {
        if self.selected_notes.is_empty() && self.selected_note.is_some() {
            1
        } else {
            self.selected_notes.len()
        }
    }

    pub(crate) fn marquee_rect(&self) -> Option<Rect> {
        let PianoDrag::Marquee { start, current, .. } = self.drag.as_ref()? else {
            return None;
        };
        Some(Rect::from_points(*start, *current))
    }

    pub(crate) fn active_time_selection_rect(&self, grid: Rect) -> Option<Rect> {
        if let Some(PianoDrag::TimeSelection { start, current }) = self.drag.as_ref() {
            return Some(self.time_selection_rect_for_points(grid, *start, *current));
        }
        if let Some(PianoDrag::MoveTimeSelection {
            source_start_beat,
            source_end_beat,
            grab_beat,
            current,
        }) = self.drag.as_ref()
        {
            let (start, end) = self.moved_time_selection_beats(
                *source_start_beat,
                *source_end_beat,
                *grab_beat,
                *current,
                grid,
            );
            return Some(self.time_selection_rect_for_beats(grid, start, end));
        }
        let (start_beat, end_beat) = self.time_selection?;
        Some(self.time_selection_rect_for_beats(grid, start_beat, end_beat))
    }

    pub(crate) fn moving_time_selection_source_rect(&self, grid: Rect) -> Option<Rect> {
        let PianoDrag::MoveTimeSelection {
            source_start_beat,
            source_end_beat,
            ..
        } = self.drag.as_ref()?
        else {
            return None;
        };
        Some(self.time_selection_rect_for_beats(grid, *source_start_beat, *source_end_beat))
    }

    pub(crate) fn moving_time_selection_clears_source(&self) -> bool {
        matches!(self.drag, Some(PianoDrag::MoveTimeSelection { .. }))
            && !self.pointer_modifiers.command
    }

    pub(crate) fn time_selection_contains(&self, grid: Rect, position: Point) -> bool {
        self.time_selection
            .map(|(start, end)| self.time_selection_rect_for_beats(grid, start, end))
            .is_some_and(|rect| rect.contains(position))
    }

    fn time_selection_rect_for_beats(&self, grid: Rect, start_beat: f32, end_beat: f32) -> Rect {
        beat_range_rect_view(grid, self.viewport, start_beat, end_beat)
    }

    pub(crate) fn moved_time_selection_beats(
        &self,
        source_start_beat: f32,
        source_end_beat: f32,
        grab_beat: f32,
        current: Point,
        grid: Rect,
    ) -> (f32, f32) {
        let length = (source_end_beat - source_start_beat).abs();
        let delta = self.beat_for_position(grid, current) - grab_beat;
        let target_start = (source_start_beat + delta).clamp(0.0, TOTAL_BEATS - length);
        (target_start, target_start + length)
    }

    pub(crate) fn edit_cursor_x(&self, grid: Rect) -> Option<f32> {
        match self.drag.as_ref() {
            Some(PianoDrag::TimeSelection { start, .. }) => Some(self.x_for_position(grid, *start)),
            _ => self
                .edit_cursor_beat
                .map(|beat| x_for_beat_view(grid, self.viewport, beat)),
        }
    }

    pub(crate) fn hover_cursor_x(&self, grid: Rect, position: Point) -> f32 {
        self.x_for_position(grid, position)
    }

    pub(crate) fn beat_for_position(&self, grid: Rect, position: Point) -> f32 {
        self.resolve_beat(beat_for_x_view(grid, self.viewport, position.x))
    }

    pub(crate) fn resolve_beat(&self, beat: f32) -> f32 {
        let beat = beat.clamp(0.0, TOTAL_BEATS);
        if self.snap_enabled {
            quantize_beat(beat)
        } else {
            beat
        }
    }

    pub(crate) fn x_for_position(&self, grid: Rect, position: Point) -> f32 {
        x_for_beat_view(grid, self.viewport, self.beat_for_position(grid, position))
            .clamp(grid.min.x, grid.max.x)
    }

    fn time_selection_rect_for_points(&self, grid: Rect, start: Point, current: Point) -> Rect {
        let x0 = self.x_for_position(grid, start);
        let x1 = self.x_for_position(grid, current);
        Rect::from_min_max(
            Point::new(x0.min(x1), grid.min.y),
            Point::new(x0.max(x1), grid.max.y),
        )
    }

    pub(crate) fn keyboard_pitch_rect(&self, keyboard: Rect, pitch: i32) -> Rect {
        pitch_layout(keyboard, self.viewport).pitch_rect(pitch)
    }
}
