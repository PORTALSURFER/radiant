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

pub(crate) use model::PianoRollState;
pub(crate) use view::project_surface;
pub(crate) use widget::PianoRollWidget;

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
    SetVelocity {
        ids: Vec<u32>,
        velocity: f32,
    },
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
        AppMessage::Reset => state.reset(),
        AppMessage::Roll(message) => state.apply_roll_message(message),
    }
}
