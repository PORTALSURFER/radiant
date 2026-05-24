use super::{
    DEFAULT_NOTE_LENGTH, LOW_PITCH, PITCH_ROWS, PianoRollMessage, TOTAL_BEATS,
    geometry::pitch_label, geometry::quantize_beat, geometry::synthetic_velocity,
};

#[derive(Clone, Debug)]
pub(crate) struct PianoRollState {
    pub(crate) running: bool,
    pub(crate) frame: u64,
    pub(crate) playhead_beat: f32,
    pub(crate) selected_note: Option<u32>,
    pub(crate) notes: Vec<PianoNote>,
}

impl Default for PianoRollState {
    fn default() -> Self {
        Self {
            running: true,
            frame: 0,
            playhead_beat: 0.0,
            selected_note: Some(2),
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
        let selected = self
            .selected_note
            .and_then(|id| self.notes.iter().find(|note| note.id == id))
            .map(selected_note_status)
            .unwrap_or_else(|| "no note".into());
        format!(
            "{transport} | playhead {:.2} | selected {selected} | synthetic GUI data",
            self.playhead_beat
        )
    }

    pub(crate) fn apply_roll_message(&mut self, message: PianoRollMessage) {
        match message {
            PianoRollMessage::SelectNote(id) => self.selected_note = Some(id),
            PianoRollMessage::CreateNote { pitch, start_beat } => {
                self.create_note(pitch, start_beat)
            }
            PianoRollMessage::MoveNote {
                id,
                pitch,
                start_beat,
            } => {
                self.move_note(id, pitch, start_beat);
            }
            PianoRollMessage::ResizeNote {
                id,
                start_beat,
                length_beats,
            } => self.resize_note(id, start_beat, length_beats),
            PianoRollMessage::DeleteSelected => self.delete_selected(),
        }
    }

    fn create_note(&mut self, pitch: i32, start_beat: f32) {
        let id = self.next_note_id();
        self.notes.push(PianoNote::new(
            id,
            clamp_pitch(pitch),
            quantize_beat(start_beat),
            DEFAULT_NOTE_LENGTH,
            synthetic_velocity(id),
        ));
        self.selected_note = Some(id);
    }

    fn move_note(&mut self, id: u32, pitch: i32, start_beat: f32) {
        if let Some(note) = self.notes.iter_mut().find(|note| note.id == id) {
            note.pitch = clamp_pitch(pitch);
            note.start_beat = quantize_beat(start_beat).clamp(0.0, TOTAL_BEATS - note.length_beats);
            self.selected_note = Some(id);
        }
    }

    fn resize_note(&mut self, id: u32, start_beat: f32, length_beats: f32) {
        if let Some(note) = self.notes.iter_mut().find(|note| note.id == id) {
            let end_beat = (start_beat + length_beats).clamp(0.25, TOTAL_BEATS);
            note.start_beat = quantize_beat(start_beat).clamp(0.0, end_beat - 0.25);
            note.length_beats = quantize_beat(end_beat - note.start_beat).clamp(0.25, 4.0);
            self.selected_note = Some(id);
        }
    }

    fn delete_selected(&mut self) {
        if let Some(id) = self.selected_note.take() {
            self.notes.retain(|note| note.id != id);
        }
    }

    fn next_note_id(&self) -> u32 {
        self.notes
            .iter()
            .map(|note| note.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PianoNote {
    pub(crate) id: u32,
    pub(crate) pitch: i32,
    pub(crate) start_beat: f32,
    pub(crate) length_beats: f32,
    pub(crate) velocity: f32,
}

impl PianoNote {
    const fn new(id: u32, pitch: i32, start_beat: f32, length_beats: f32, velocity: f32) -> Self {
        Self {
            id,
            pitch,
            start_beat,
            length_beats,
            velocity,
        }
    }

    pub(crate) fn end_beat(self) -> f32 {
        self.start_beat + self.length_beats
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

fn clamp_pitch(pitch: i32) -> i32 {
    pitch.clamp(LOW_PITCH, LOW_PITCH + PITCH_ROWS as i32 - 1)
}
