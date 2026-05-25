use super::{
    PianoNote, PianoRollState, PianoRollViewport, STRESS_NOTE_COUNT, editing::clamp_pitch,
};
use crate::piano_roll::{
    LOW_PITCH, PITCH_ROWS, PianoRollMessage, PianoRollTool, TOTAL_BEATS,
    geometry::synthetic_velocity,
};

impl PianoRollState {
    pub(crate) fn apply_roll_message(&mut self, message: PianoRollMessage) {
        match message {
            PianoRollMessage::SelectNote(id) => self.replace_selection([id]),
            PianoRollMessage::SelectPitch(pitch) => self.select_pitch(pitch),
            PianoRollMessage::SelectNotes { ids, mode } => self.select_notes(ids, mode),
            PianoRollMessage::CreateNote {
                pitch,
                start_beat,
                length_beats,
            } => self.create_note(pitch, start_beat, length_beats),
            PianoRollMessage::MoveNote {
                id,
                pitch,
                start_beat,
            } => {
                self.move_note(id, pitch, start_beat);
            }
            PianoRollMessage::MoveNotes {
                ids,
                pitch_delta,
                beat_delta,
            } => self.move_notes(ids, pitch_delta, beat_delta),
            PianoRollMessage::ResizeNote {
                id,
                start_beat,
                length_beats,
            } => self.resize_note(id, start_beat, length_beats),
            PianoRollMessage::SetVelocity { ids, velocity } => self.set_velocity(ids, velocity),
            PianoRollMessage::PanViewport {
                beat_delta,
                pitch_delta,
            } => self.viewport.pan(beat_delta, pitch_delta),
            PianoRollMessage::ZoomTime { factor } => self.viewport.zoom_time(factor),
            PianoRollMessage::ZoomPitch { rows_delta } => self.viewport.zoom_pitch(rows_delta),
            PianoRollMessage::ZoomViewport {
                time_factor,
                rows_delta,
            } => self.viewport.zoom(time_factor, rows_delta),
            PianoRollMessage::SetTool(tool) => self.tool = tool,
            PianoRollMessage::ToggleStressNotes => self.toggle_stress_notes(),
            PianoRollMessage::DeleteSelected => self.delete_selected(),
        }
    }

    fn select_pitch(&mut self, pitch: i32) {
        self.selected_pitch = Some(clamp_pitch(pitch));
    }

    fn toggle_stress_notes(&mut self) {
        if self.notes.len() >= STRESS_NOTE_COUNT {
            *self = Self::default();
            return;
        }
        self.notes = synthetic_stress_notes();
        self.viewport = PianoRollViewport::default();
        self.tool = PianoRollTool::Select;
        self.replace_selection(std::iter::empty());
    }
}

fn synthetic_stress_notes() -> Vec<PianoNote> {
    let columns = 128;
    (0..STRESS_NOTE_COUNT)
        .map(|index| {
            let pitch = LOW_PITCH + (index % PITCH_ROWS) as i32;
            let column = (index / PITCH_ROWS) % columns;
            let layer = index / (PITCH_ROWS * columns);
            let start_beat = column as f32 * TOTAL_BEATS / columns as f32 + layer as f32 * 0.018;
            PianoNote::new(
                index as u32 + 1,
                pitch,
                start_beat.min(TOTAL_BEATS - 0.04),
                0.075,
                synthetic_velocity(index as u32 + 1),
            )
        })
        .collect()
}
