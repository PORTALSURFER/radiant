#[path = "model/editing.rs"]
mod editing;
#[path = "model/note.rs"]
mod note;
#[path = "model/selection.rs"]
mod selection;
#[path = "model/update.rs"]
mod update;
#[path = "model/viewport.rs"]
mod viewport;

pub(crate) use note::PianoNote;
pub(crate) use viewport::PianoRollViewport;

use super::{PianoRollTool, TOTAL_BEATS, geometry::pitch_label};
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
}

fn selected_note_status(note: &PianoNote) -> String {
    format!(
        "{} beat {:.2} len {:.2}",
        pitch_label(note.pitch),
        note.start_beat,
        note.length_beats
    )
}
