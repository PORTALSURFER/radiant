use radiant::prelude::*;
use radiant::widgets::PaintBounds;
use radiant::widgets::PointerModifiers;

use super::{
    PianoRollTool,
    drag::PianoDrag,
    geometry::{beat_for_x_view, quantize_beat, row_height_for, x_for_beat_view, y_for_pitch_view},
    model::{PianoNote, PianoRollViewport},
};

#[path = "widget/input.rs"]
mod input;
#[path = "widget/navigation_input.rs"]
mod navigation_input;
#[path = "widget/pointer_input.rs"]
mod pointer_input;
#[path = "widget/selection_input.rs"]
mod selection_input;
#[path = "widget/velocity_input.rs"]
mod velocity_input;

#[derive(Clone, Debug)]
pub(crate) struct PianoRollWidget {
    pub(super) common: WidgetCommon,
    pub(super) notes: Vec<PianoNote>,
    pub(super) selected_note: Option<u32>,
    pub(super) selected_notes: Vec<u32>,
    pub(super) selected_pitch: Option<i32>,
    pub(super) edit_cursor_beat: Option<f32>,
    pub(super) time_selection: Option<(f32, f32)>,
    pub(super) snap_enabled: bool,
    pub(super) playhead_beat: f32,
    pub(super) viewport: PianoRollViewport,
    pub(super) tool: PianoRollTool,
    pub(crate) hover_note: Option<u32>,
    pub(crate) hover_pitch: Option<i32>,
    pub(crate) active_pitch: Option<i32>,
    pub(super) hover_position: Option<Point>,
    pub(super) pointer_modifiers: PointerModifiers,
    pub(super) drag: Option<PianoDrag>,
}

impl PianoRollWidget {
    pub(crate) fn new(
        notes: Vec<PianoNote>,
        selected_note: Option<u32>,
        selected_notes: Vec<u32>,
        selected_pitch: Option<i32>,
        edit_cursor_beat: Option<f32>,
        time_selection: Option<(f32, f32)>,
        snap_enabled: bool,
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
            selected_notes: sorted_unique_ids(selected_notes),
            selected_pitch,
            edit_cursor_beat,
            time_selection,
            snap_enabled,
            playhead_beat,
            viewport,
            tool,
            hover_note: None,
            hover_pitch: None,
            active_pitch: None,
            hover_position: None,
            pointer_modifiers: PointerModifiers::default(),
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
        Some(rect_from_points(*start, *current))
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
        let x0 = x_for_beat_view(grid, self.viewport, start_beat);
        let x1 = x_for_beat_view(grid, self.viewport, end_beat);
        Rect::from_min_max(
            Point::new(x0.min(x1), grid.min.y),
            Point::new(x0.max(x1), grid.max.y),
        )
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
        let target_start = (source_start_beat + delta).clamp(0.0, super::TOTAL_BEATS - length);
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
        let beat = beat.clamp(0.0, super::TOTAL_BEATS);
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
            PianoDrag::VelocityMarquee { .. } => return None,
            PianoDrag::TimeSelection { .. } | PianoDrag::MoveTimeSelection { .. } => return None,
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
                .preview_note(grid, self.viewport, position, source, self.snap_enabled),
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
                    self.snap_enabled,
                )
            })
            .collect()
    }

    pub(crate) fn time_slice_preview_notes(&self, grid: Rect) -> Vec<PianoNote> {
        let Some(PianoDrag::MoveTimeSelection {
            source_start_beat,
            source_end_beat,
            grab_beat,
            current,
        }) = self.drag.as_ref()
        else {
            return Vec::new();
        };
        let (source_start, source_end) = if source_start_beat <= source_end_beat {
            (*source_start_beat, *source_end_beat)
        } else {
            (*source_end_beat, *source_start_beat)
        };
        let (target_start, _) =
            self.moved_time_selection_beats(source_start, source_end, *grab_beat, *current, grid);
        let beat_delta = target_start - source_start;
        self.notes
            .iter()
            .copied()
            .filter_map(|note| clipped_note_for_range(note, source_start, source_end, beat_delta))
            .collect()
    }
}

fn sorted_unique_ids(mut ids: Vec<u32>) -> Vec<u32> {
    SelectionSet::normalize_vec(&mut ids);
    ids
}

fn rect_from_points(a: Point, b: Point) -> Rect {
    Rect::from_min_max(
        Point::new(a.x.min(b.x), a.y.min(b.y)),
        Point::new(a.x.max(b.x), a.y.max(b.y)),
    )
}

fn clipped_note_for_range(
    note: PianoNote,
    start_beat: f32,
    end_beat: f32,
    beat_delta: f32,
) -> Option<PianoNote> {
    if note.start_beat >= end_beat || note.end_beat() <= start_beat {
        return None;
    }
    let clipped_start = note.start_beat.max(start_beat);
    let clipped_end = note.end_beat().min(end_beat);
    let length_beats = clipped_end - clipped_start;
    if length_beats <= f32::EPSILON {
        return None;
    }
    Some(PianoNote {
        start_beat: clipped_start + beat_delta,
        length_beats,
        ..note
    })
}
