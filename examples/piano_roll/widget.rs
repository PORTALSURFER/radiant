use radiant::prelude::*;
use radiant::widgets::PaintBounds;
use radiant::widgets::PointerModifiers;

use super::{
    PianoRollTool,
    drag::PianoDrag,
    model::{PianoNote, PianoRollState, PianoRollViewport},
};

#[path = "widget/geometry_view.rs"]
mod geometry_view;
#[path = "widget/input.rs"]
mod input;
#[path = "widget/navigation_input.rs"]
mod navigation_input;
#[path = "widget/note_input.rs"]
mod note_input;
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
    pub(crate) hover_note_resize_edge: Option<NoteResizeEdge>,
    pub(crate) hover_velocity_note: Option<u32>,
    pub(crate) hover_pitch: Option<i32>,
    pub(crate) active_pitch: Option<i32>,
    pub(super) hover_position: Option<Point>,
    pub(super) pointer_modifiers: PointerModifiers,
    pub(super) drag: Option<PianoDrag>,
}

#[derive(Clone, Debug)]
pub(crate) struct PianoRollWidgetParts {
    pub(crate) notes: Vec<PianoNote>,
    pub(crate) selected_note: Option<u32>,
    pub(crate) selected_notes: Vec<u32>,
    pub(crate) selected_pitch: Option<i32>,
    pub(crate) edit_cursor_beat: Option<f32>,
    pub(crate) time_selection: Option<(f32, f32)>,
    pub(crate) snap_enabled: bool,
    pub(crate) playhead_beat: f32,
    pub(crate) viewport: PianoRollViewport,
    pub(crate) tool: PianoRollTool,
}

impl PianoRollWidgetParts {
    pub(crate) fn from_state(state: &PianoRollState) -> Self {
        Self {
            notes: state.notes.clone(),
            selected_note: state.selected_note,
            selected_notes: state.selected_notes.clone(),
            selected_pitch: state.selected_pitch,
            edit_cursor_beat: state.edit_cursor_beat,
            time_selection: state.time_selection,
            snap_enabled: state.snap_enabled,
            playhead_beat: state.playhead_beat,
            viewport: state.viewport,
            tool: state.tool,
        }
    }
}

impl PianoRollWidget {
    pub(crate) fn new(parts: PianoRollWidgetParts) -> Self {
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
            notes: parts.notes,
            selected_note: parts.selected_note,
            selected_notes: sorted_unique_ids(parts.selected_notes),
            selected_pitch: parts.selected_pitch,
            edit_cursor_beat: parts.edit_cursor_beat,
            time_selection: parts.time_selection,
            snap_enabled: parts.snap_enabled,
            playhead_beat: parts.playhead_beat,
            viewport: parts.viewport,
            tool: parts.tool,
            hover_note: None,
            hover_note_resize_edge: None,
            hover_velocity_note: None,
            hover_pitch: None,
            active_pitch: None,
            hover_position: None,
            pointer_modifiers: PointerModifiers::default(),
            drag: None,
        }
    }

    pub(crate) fn drag_preview_note(&self, grid: Rect, position: Point) -> Option<PianoNote> {
        let drag = self.drag.as_ref()?;
        let source = match drag {
            PianoDrag::Pan { .. } | PianoDrag::Marquee { .. } => return None,
            PianoDrag::VelocityMarquee { .. } => return None,
            PianoDrag::TimeSelection { .. } | PianoDrag::MoveTimeSelection { .. } => return None,
            PianoDrag::Velocity { .. } | PianoDrag::VelocityRelative { .. } => return None,
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

pub(crate) const NOTE_RESIZE_EDGE_WIDTH: f32 = 8.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NoteResizeEdge {
    Start,
    End,
}

fn sorted_unique_ids(mut ids: Vec<u32>) -> Vec<u32> {
    SelectionSet::normalize_vec(&mut ids);
    ids
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
