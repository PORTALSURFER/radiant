#[path = "drag.rs"]
mod drag;
#[path = "geometry.rs"]
mod geometry;
#[path = "model.rs"]
mod model;
#[path = "paint.rs"]
mod paint;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
#[path = "view.rs"]
mod view;
#[path = "widget.rs"]
mod widget;
#[path = "widget_paint.rs"]
mod widget_paint;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub(crate) use model::PianoRollState;
pub(crate) use view::project_surface;
pub(crate) use widget::{PianoRollWidget, PianoRollWidgetParts};

pub(crate) const PIANO_ROLL_WIDGET_ID: u64 = 92;
pub(crate) const STATUS_WIDGET_ID: u64 = 93;
pub(crate) const TOTAL_BEATS: f32 = 16.0;
pub(crate) const PITCH_ROWS: usize = 24;
pub(crate) const LOW_PITCH: i32 = 48;
pub(crate) const DATA_SOURCE_NOTE: &str = "without_midi_or_dsp";

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum AppMessage {
    Frame,
    ToggleRun,
    Reset,
    Undo,
    Redo,
    Roll(PianoRollMessage),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PianoRollMessage {
    SelectNote(u32),
    SelectPitch(i32),
    SelectNotes {
        ids: Vec<u32>,
        mode: NoteSelectionMode,
    },
    CreateNote {
        pitch: i32,
        start_beat: f32,
        length_beats: f32,
    },
    MoveNote {
        id: u32,
        pitch: i32,
        start_beat: f32,
    },
    MoveNotes {
        ids: Vec<u32>,
        pitch_delta: i32,
        beat_delta: f32,
    },
    ResizeNote {
        id: u32,
        start_beat: f32,
        length_beats: f32,
    },
    SetVelocities {
        velocities: Vec<(u32, f32)>,
    },
    SetCursor {
        beat: f32,
    },
    SetTimeSelection {
        start_beat: f32,
        end_beat: f32,
    },
    MoveTimeSelection {
        source_start_beat: f32,
        source_end_beat: f32,
        target_start_beat: f32,
    },
    CopyTimeSelection {
        source_start_beat: f32,
        source_end_beat: f32,
        target_start_beat: f32,
    },
    ToggleSnap,
    PanViewport {
        beat_delta: f32,
        pitch_delta: i32,
    },
    ZoomTime {
        factor: f32,
    },
    ZoomPitch {
        rows_delta: i32,
    },
    ZoomViewport {
        time_factor: Option<f32>,
        rows_delta: i32,
    },
    SetTool(PianoRollTool),
    ToggleStressNotes,
    DeleteSelected,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PianoRollTool {
    Paint,
    Select,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NoteSelectionMode {
    Replace,
    Add,
    Toggle,
}

pub(crate) fn update(state: &mut PianoRollState, message: AppMessage) {
    match message {
        AppMessage::Frame => state.tick(),
        AppMessage::ToggleRun => {
            state.running = !state.running;
        }
        AppMessage::Reset => {
            let before = state.snapshot();
            state.reset();
            let after = state.snapshot();
            state
                .history
                .register_change("Reset piano roll", before, &after);
        }
        AppMessage::Undo => {
            let current = state.snapshot();
            if let Some(transition) = state.history.undo(&current) {
                state.restore_snapshot(transition.state);
            }
        }
        AppMessage::Redo => {
            let current = state.snapshot();
            if let Some(transition) = state.history.redo(&current) {
                state.restore_snapshot(transition.state);
            }
        }
        AppMessage::Roll(message) => apply_undoable_roll_message(state, message),
    }
}

fn apply_undoable_roll_message(state: &mut PianoRollState, message: PianoRollMessage) {
    let registration = undo_registration_for(&message);
    if let Some((_, Some(merge_key))) = registration.as_ref()
        && state.history.can_coalesce_change(merge_key)
    {
        state.apply_roll_message(message);
        return;
    }
    let before = registration.as_ref().map(|_| state.snapshot());
    state.apply_roll_message(message);
    if let (Some((label, merge_key)), Some(before)) = (registration, before) {
        let after = state.snapshot();
        if let Some(merge_key) = merge_key {
            state
                .history
                .register_change_coalescing(label, merge_key, before, &after);
        } else {
            state.history.register_change(label, before, &after);
        }
    }
}

fn undo_registration_for(message: &PianoRollMessage) -> Option<(&'static str, Option<String>)> {
    let label = match message {
        PianoRollMessage::SelectNote(_) => "Select note",
        PianoRollMessage::SelectPitch(_) => "Select pitch",
        PianoRollMessage::SelectNotes { .. } => "Select notes",
        PianoRollMessage::CreateNote { .. } => "Create note",
        PianoRollMessage::MoveNote { .. } | PianoRollMessage::MoveNotes { .. } => "Move notes",
        PianoRollMessage::ResizeNote { .. } => "Resize note",
        PianoRollMessage::SetVelocities { velocities } => {
            return Some(("Change velocity", Some(velocity_merge_key(velocities))));
        }
        PianoRollMessage::SetCursor { .. } => "Set cursor",
        PianoRollMessage::SetTimeSelection { .. } => "Set time selection",
        PianoRollMessage::MoveTimeSelection { .. } => "Move time selection",
        PianoRollMessage::CopyTimeSelection { .. } => "Copy time selection",
        PianoRollMessage::ToggleSnap => "Toggle snap",
        PianoRollMessage::PanViewport { .. } => "Pan viewport",
        PianoRollMessage::ZoomTime { .. }
        | PianoRollMessage::ZoomPitch { .. }
        | PianoRollMessage::ZoomViewport { .. } => "Zoom viewport",
        PianoRollMessage::SetTool(_) => "Change tool",
        PianoRollMessage::ToggleStressNotes => "Toggle stress notes",
        PianoRollMessage::DeleteSelected => "Delete selected notes",
    };
    Some((label, None))
}

fn velocity_merge_key(velocities: &[(u32, f32)]) -> String {
    let mut ids = velocities.iter().map(|(id, _)| *id).collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    let mut hasher = DefaultHasher::new();
    ids.hash(&mut hasher);
    format!("velocity:{}:{:016x}", ids.len(), hasher.finish())
}
