use super::{
    LOW_PITCH, NoteSelectionMode, PITCH_ROWS, PianoRollMessage, PianoRollTool, TOTAL_BEATS,
    geometry::pitch_label, geometry::quantize_beat, geometry::synthetic_velocity,
};

const MIN_VISIBLE_BEATS: f32 = 4.0;
const MIN_VISIBLE_PITCHES: usize = 8;
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

    fn create_note(&mut self, pitch: i32, start_beat: f32, length_beats: f32) {
        let id = self.next_note_id();
        let note = PianoNote::new(
            id,
            clamp_pitch(pitch),
            quantize_beat(start_beat),
            quantized_length(length_beats),
            synthetic_velocity(id),
        );
        self.cut_notes_for(note, &[id]);
        self.notes.push(note);
        self.selected_note = Some(id);
        self.selected_notes.clear();
        self.selected_notes.push(id);
    }

    fn move_note(&mut self, id: u32, pitch: i32, start_beat: f32) {
        if let Some(note) = self.notes.iter().copied().find(|note| note.id == id) {
            let mut moved = note;
            moved.pitch = clamp_pitch(pitch);
            moved.start_beat =
                quantize_beat(start_beat).clamp(0.0, TOTAL_BEATS - moved.length_beats);
            self.cut_notes_for(moved, &[id]);
        }
        if let Some(note) = self.notes.iter_mut().find(|note| note.id == id) {
            note.pitch = clamp_pitch(pitch);
            note.start_beat = quantize_beat(start_beat).clamp(0.0, TOTAL_BEATS - note.length_beats);
            self.replace_selection([id]);
        }
    }

    fn resize_note(&mut self, id: u32, start_beat: f32, length_beats: f32) {
        if let Some(note) = self.notes.iter().copied().find(|note| note.id == id) {
            let end_beat = (start_beat + length_beats).clamp(0.25, TOTAL_BEATS);
            let mut resized = note;
            resized.start_beat = quantize_beat(start_beat).clamp(0.0, end_beat - 0.25);
            resized.length_beats = quantized_length(end_beat - resized.start_beat);
            self.cut_notes_for(resized, &[id]);
        }
        if let Some(note) = self.notes.iter_mut().find(|note| note.id == id) {
            let end_beat = (start_beat + length_beats).clamp(0.25, TOTAL_BEATS);
            note.start_beat = quantize_beat(start_beat).clamp(0.0, end_beat - 0.25);
            note.length_beats = quantized_length(end_beat - note.start_beat);
            self.replace_selection([id]);
        }
    }

    fn move_notes(&mut self, ids: Vec<u32>, pitch_delta: i32, beat_delta: f32) {
        let mut ids = ids;
        ids.retain(|id| self.notes.iter().any(|note| note.id == *id));
        ids.sort_unstable();
        ids.dedup();
        if ids.is_empty() {
            return;
        }
        let moved_notes = self
            .notes
            .iter()
            .copied()
            .filter(|note| ids.binary_search(&note.id).is_ok())
            .map(|note| PianoNote {
                pitch: clamp_pitch(note.pitch + pitch_delta),
                start_beat: quantize_beat(note.start_beat + beat_delta)
                    .clamp(0.0, TOTAL_BEATS - note.length_beats),
                ..note
            })
            .collect::<Vec<_>>();
        for moved in &moved_notes {
            self.cut_notes_for(*moved, &ids);
        }
        for moved in moved_notes {
            if let Some(note) = self.notes.iter_mut().find(|note| note.id == moved.id) {
                note.pitch = moved.pitch;
                note.start_beat = moved.start_beat;
            }
        }
        self.replace_selection(ids);
    }

    fn cut_notes_for(&mut self, cutter: PianoNote, except_ids: &[u32]) {
        let existing_notes = std::mem::take(&mut self.notes);
        let mut next_split_id = existing_notes
            .iter()
            .map(|note| note.id)
            .max()
            .unwrap_or(0)
            .max(cutter.id)
            .saturating_add(1);
        let mut split_notes = Vec::new();
        for note in existing_notes {
            if except_ids.contains(&note.id) || note.pitch != cutter.pitch {
                self.notes.push(note);
                continue;
            }
            let note_start = note.start_beat;
            let note_end = note.end_beat();
            let cut_start = cutter.start_beat;
            let cut_end = cutter.end_beat();
            if cut_end <= note_start || cut_start >= note_end {
                self.notes.push(note);
                continue;
            }
            if cut_start > note_start {
                split_notes.push(PianoNote::new(
                    next_split_id,
                    note.pitch,
                    note_start,
                    cut_start - note_start,
                    note.velocity,
                ));
                next_split_id = next_split_id.saturating_add(1);
            }
            if cut_end < note_end {
                split_notes.push(PianoNote::new(
                    next_split_id,
                    note.pitch,
                    cut_end,
                    note_end - cut_end,
                    note.velocity,
                ));
                next_split_id = next_split_id.saturating_add(1);
            }
        }
        self.notes.extend(split_notes);
        self.notes.sort_by(|a, b| {
            a.start_beat
                .total_cmp(&b.start_beat)
                .then(a.pitch.cmp(&b.pitch))
        });
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

    fn set_velocity(&mut self, ids: Vec<u32>, velocity: f32) {
        let velocity = velocity.clamp(0.0, 1.0);
        for note in &mut self.notes {
            if ids.contains(&note.id) {
                note.velocity = velocity;
            }
        }
        if !ids.is_empty() {
            self.select_notes(ids, NoteSelectionMode::Replace);
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PianoRollViewport {
    pub(crate) beat_start: f32,
    pub(crate) visible_beats: f32,
    pub(crate) pitch_start: i32,
    pub(crate) visible_pitches: usize,
}

impl Default for PianoRollViewport {
    fn default() -> Self {
        Self {
            beat_start: 0.0,
            visible_beats: TOTAL_BEATS,
            pitch_start: LOW_PITCH,
            visible_pitches: PITCH_ROWS,
        }
    }
}

impl PianoRollViewport {
    pub(crate) fn beat_end(self) -> f32 {
        (self.beat_start + self.visible_beats).min(TOTAL_BEATS)
    }

    pub(crate) fn pitch_end(self) -> i32 {
        self.pitch_start + self.visible_pitches as i32 - 1
    }

    pub(crate) fn row_count(self) -> usize {
        self.visible_pitches
    }

    fn pan(&mut self, beat_delta: f32, pitch_delta: i32) {
        let panned = self.panned(beat_delta, pitch_delta);
        self.beat_start = panned.beat_start;
        self.pitch_start = panned.pitch_start;
    }

    pub(crate) fn panned(self, beat_delta: f32, pitch_delta: i32) -> Self {
        let max_pitch_start = Self::max_pitch_start(self.visible_pitches);
        Self {
            beat_start: (self.beat_start + beat_delta)
                .clamp(0.0, (TOTAL_BEATS - self.visible_beats).max(0.0)),
            pitch_start: (self.pitch_start + pitch_delta).clamp(LOW_PITCH, max_pitch_start),
            ..self
        }
    }

    pub(crate) fn pan_delta_to(self, target: Self) -> (f32, i32) {
        (
            target.beat_start - self.beat_start,
            target.pitch_start - self.pitch_start,
        )
    }

    fn max_pitch_start(visible_pitches: usize) -> i32 {
        LOW_PITCH + PITCH_ROWS as i32 - visible_pitches as i32
    }

    fn zoom_time(&mut self, factor: f32) {
        let center = self.beat_start + self.visible_beats * 0.5;
        self.visible_beats = (self.visible_beats * factor).clamp(MIN_VISIBLE_BEATS, TOTAL_BEATS);
        self.beat_start =
            (center - self.visible_beats * 0.5).clamp(0.0, TOTAL_BEATS - self.visible_beats);
    }

    pub(crate) fn can_zoom_time(self, factor: f32) -> bool {
        let next_visible = (self.visible_beats * factor).clamp(MIN_VISIBLE_BEATS, TOTAL_BEATS);
        (next_visible - self.visible_beats).abs() > f32::EPSILON
    }

    fn zoom(&mut self, time_factor: Option<f32>, rows_delta: i32) {
        if let Some(factor) = time_factor {
            self.zoom_time(factor);
        }
        self.zoom_pitch(rows_delta);
    }

    fn zoom_pitch(&mut self, rows_delta: i32) {
        let center = self.pitch_start + self.visible_pitches as i32 / 2;
        self.visible_pitches = (self.visible_pitches as i32 + rows_delta)
            .clamp(MIN_VISIBLE_PITCHES as i32, PITCH_ROWS as i32)
            as usize;
        let max_pitch_start = Self::max_pitch_start(self.visible_pitches);
        self.pitch_start =
            (center - self.visible_pitches as i32 / 2).clamp(LOW_PITCH, max_pitch_start);
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

fn quantized_length(length_beats: f32) -> f32 {
    quantize_beat(length_beats).clamp(0.25, 4.0)
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
