use super::{PianoNote, PianoRollState};
use crate::piano_roll::{
    LOW_PITCH, NoteSelectionMode, PITCH_ROWS, TOTAL_BEATS,
    geometry::{quantize_beat, synthetic_velocity},
};
use radiant::prelude::SelectionSet;

impl PianoRollState {
    pub(super) fn create_note(&mut self, pitch: i32, start_beat: f32, length_beats: f32) {
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

    pub(super) fn move_note(&mut self, id: u32, pitch: i32, start_beat: f32) {
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

    pub(super) fn resize_note(&mut self, id: u32, start_beat: f32, length_beats: f32) {
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

    pub(super) fn move_notes(&mut self, ids: Vec<u32>, pitch_delta: i32, beat_delta: f32) {
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

    pub(super) fn set_velocities(&mut self, mut velocities: Vec<(u32, f32)>) {
        if !SelectionSet::slice_is_sorted_unique_by_key(&velocities, |(id, _)| *id) {
            velocities.sort_unstable_by_key(|(id, _)| *id);
            velocities.dedup_by_key(|(id, _)| *id);
        }
        let mut ids = Vec::with_capacity(velocities.len());
        for note in &mut self.notes {
            if let Ok(index) = velocities.binary_search_by_key(&note.id, |(id, _)| *id) {
                note.velocity = velocities[index].1.clamp(0.0, 1.0);
                ids.push(note.id);
            }
        }
        if ids.is_empty() {
            return;
        }
        SelectionSet::normalize_vec(&mut ids);
        if self.selected_notes == ids {
            self.selected_note = self.selected_notes.first().copied();
        } else {
            self.select_notes(ids, NoteSelectionMode::Replace);
        }
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
            next_split_id =
                self.split_note_around_cutter(note, cutter, next_split_id, &mut split_notes);
        }
        self.notes.extend(split_notes);
        self.notes.sort_by(|a, b| {
            a.start_beat
                .total_cmp(&b.start_beat)
                .then(a.pitch.cmp(&b.pitch))
        });
    }

    fn split_note_around_cutter(
        &mut self,
        note: PianoNote,
        cutter: PianoNote,
        mut next_split_id: u32,
        split_notes: &mut Vec<PianoNote>,
    ) -> u32 {
        let note_start = note.start_beat;
        let note_end = note.end_beat();
        let cut_start = cutter.start_beat;
        let cut_end = cutter.end_beat();
        if cut_end <= note_start || cut_start >= note_end {
            self.notes.push(note);
            return next_split_id;
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
        next_split_id
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

pub(super) fn clamp_pitch(pitch: i32) -> i32 {
    pitch.clamp(LOW_PITCH, LOW_PITCH + PITCH_ROWS as i32 - 1)
}

fn quantized_length(length_beats: f32) -> f32 {
    quantize_beat(length_beats).clamp(0.25, 4.0)
}
