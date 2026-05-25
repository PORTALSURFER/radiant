#[path = "model/editing.rs"]
mod editing;
#[path = "model/note.rs"]
mod note;
#[path = "model/viewport.rs"]
mod viewport;

pub(crate) use note::PianoNote;
pub(crate) use viewport::PianoRollViewport;

use super::{
    LOW_PITCH, NoteSelectionMode, PITCH_ROWS, PianoRollMessage, PianoRollTool, TOTAL_BEATS,
    geometry::{pitch_label, synthetic_velocity},
};
use editing::clamp_pitch;
pub(crate) const STRESS_NOTE_COUNT: usize = 4096;

#[derive(Clone, Debug)]
pub(crate) struct PianoRollState {
    pub(crate) running: bool,
    pub(crate) frame: u64,
    pub(crate) playhead_beat: f32,
    pub(crate) viewport: PianoRollViewport,
    pub(crate) tool: PianoRollTool,
    pub(crate) selected_note: Option<u32>,
    pub(crate) selected_notes: Vec<u32>,
    pub(crate) selected_pitch: Option<i32>,
    pub(crate) notes: Vec<PianoNote>,
}

impl Default for PianoRollState {
    fn default() -> Self {
        Self {
            running: true,
            frame: 0,
            playhead_beat: 0.0,
            viewport: PianoRollViewport::default(),
            tool: PianoRollTool::Paint,
            selected_note: Some(2),
            selected_notes: vec![2],
            selected_pitch: None,
            notes: vec![
                PianoNote::new(1, 48, 0.0, 1.0, 0.72),
                PianoNote::new(2, 55, 1.0, 1.5, 0.82),
                PianoNote::new(3, 60, 2.75, 0.75, 0.64),
                PianoNote::new(4, 64, 3.5, 1.25, 0.76),
                PianoNote::new(5, 52, 5.0, 2.0, 0.88),
                PianoNote::new(6, 67, 7.25, 0.75, 0.68),
                PianoNote::new(7, 62, 9.0, 1.0, 0.70),
                PianoNote::new(8, 69, 10.5, 1.5, 0.84),
                PianoNote::new(9, 57, 12.5, 2.0, 0.78),
            ],
        }
    }
}

impl PianoRollState {
    pub(crate) fn tick(&mut self) {
        if !self.running {
            return;
        }
        self.frame = self.frame.saturating_add(1);
        self.playhead_beat = (self.playhead_beat + 0.055) % TOTAL_BEATS;
    }

    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn status(&self) -> String {
        let transport = if self.running { "running" } else { "paused" };
        let note_load = if self.notes.len() >= STRESS_NOTE_COUNT {
            "stress"
        } else {
            "normal"
        };
        let selected = self
            .selected_note
            .and_then(|id| self.notes.iter().find(|note| note.id == id))
            .map(selected_note_status)
            .unwrap_or_else(|| "no note".into());
        let selected_pitch = self
            .selected_pitch
            .map(pitch_label)
            .unwrap_or_else(|| "none".into());
        format!(
            "{transport} | {:?} | {note_load} {} notes | playhead {:.2} | beats {:.1}-{:.1} | pitches {}-{} | selected {} ({selected}) | lane {selected_pitch} | synthetic GUI data",
            self.tool,
            self.notes.len(),
            self.playhead_beat,
            self.viewport.beat_start,
            self.viewport.beat_end(),
            pitch_label(self.viewport.pitch_start),
            pitch_label(self.viewport.pitch_end()),
            self.selected_notes.len()
        )
    }

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

    fn delete_selected(&mut self) {
        if self.selected_notes.is_empty()
            && let Some(id) = self.selected_note
        {
            self.selected_notes.push(id);
        }
        let selected = self.selected_notes.clone();
        self.notes.retain(|note| !selected.contains(&note.id));
        self.selected_notes.clear();
        self.selected_note = None;
    }

    fn replace_selection(&mut self, ids: impl IntoIterator<Item = u32>) {
        self.selected_notes = ids
            .into_iter()
            .filter(|id| self.notes.iter().any(|note| note.id == *id))
            .collect();
        self.selected_notes.sort_unstable();
        self.selected_notes.dedup();
        self.selected_note = self.selected_notes.first().copied();
    }

    fn select_notes(&mut self, ids: Vec<u32>, mode: NoteSelectionMode) {
        match mode {
            NoteSelectionMode::Replace => self.replace_selection(ids),
            NoteSelectionMode::Add => {
                self.selected_notes.extend(ids);
                self.selected_notes
                    .retain(|id| self.notes.iter().any(|note| note.id == *id));
                self.selected_notes.sort_unstable();
                self.selected_notes.dedup();
                self.selected_note = self.selected_notes.first().copied();
            }
            NoteSelectionMode::Toggle => {
                for id in ids {
                    if let Some(index) = self
                        .selected_notes
                        .iter()
                        .position(|selected| *selected == id)
                    {
                        self.selected_notes.remove(index);
                    } else if self.notes.iter().any(|note| note.id == id) {
                        self.selected_notes.push(id);
                    }
                }
                self.selected_notes.sort_unstable();
                self.selected_note = self.selected_notes.first().copied();
            }
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

fn selected_note_status(note: &PianoNote) -> String {
    format!(
        "{} beat {:.2} len {:.2}",
        pitch_label(note.pitch),
        note.start_beat,
        note.length_beats
    )
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
