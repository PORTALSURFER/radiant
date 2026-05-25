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
            PianoRollMessage::SetVelocities { velocities } => self.set_velocities(velocities),
            PianoRollMessage::SetCursor { beat } => self.set_cursor(beat),
            PianoRollMessage::SetTimeSelection {
                start_beat,
                end_beat,
            } => self.set_time_selection(start_beat, end_beat),
            PianoRollMessage::MoveTimeSelection {
                source_start_beat,
                source_end_beat,
                target_start_beat,
            } => self.move_time_selection(source_start_beat, source_end_beat, target_start_beat),
            PianoRollMessage::CopyTimeSelection {
                source_start_beat,
                source_end_beat,
                target_start_beat,
            } => self.copy_time_selection(source_start_beat, source_end_beat, target_start_beat),
            PianoRollMessage::ToggleSnap => self.snap_enabled = !self.snap_enabled,
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

    fn set_cursor(&mut self, beat: f32) {
        self.edit_cursor_beat = Some(beat.clamp(0.0, TOTAL_BEATS));
        self.time_selection = None;
    }

    fn set_time_selection(&mut self, start_beat: f32, end_beat: f32) {
        let start = start_beat.clamp(0.0, TOTAL_BEATS);
        let end = end_beat.clamp(0.0, TOTAL_BEATS);
        if (end - start).abs() < f32::EPSILON {
            self.set_cursor(start);
            return;
        }
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        self.edit_cursor_beat = Some(start);
        self.time_selection = Some((start, end));
        self.replace_selection(std::iter::empty());
    }

    fn move_time_selection(
        &mut self,
        source_start_beat: f32,
        source_end_beat: f32,
        target_start_beat: f32,
    ) {
        self.place_time_slice(source_start_beat, source_end_beat, target_start_beat, false);
    }

    fn copy_time_selection(
        &mut self,
        source_start_beat: f32,
        source_end_beat: f32,
        target_start_beat: f32,
    ) {
        self.place_time_slice(source_start_beat, source_end_beat, target_start_beat, true);
    }

    fn place_time_slice(
        &mut self,
        source_start_beat: f32,
        source_end_beat: f32,
        target_start_beat: f32,
        copy: bool,
    ) {
        let source_start = source_start_beat.clamp(0.0, TOTAL_BEATS);
        let source_end = source_end_beat.clamp(0.0, TOTAL_BEATS);
        let (source_start, source_end) = if source_start <= source_end {
            (source_start, source_end)
        } else {
            (source_end, source_start)
        };
        let length = source_end - source_start;
        if length <= f32::EPSILON {
            self.set_cursor(source_start);
            return;
        }
        let target_start = target_start_beat.clamp(0.0, TOTAL_BEATS - length);
        let target_end = target_start + length;
        let mut next_id = self
            .notes
            .iter()
            .map(|note| note.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let source_notes = std::mem::take(&mut self.notes);
        let (mut remaining, mut slice) =
            split_notes_for_time_range(source_notes, source_start, source_end, !copy, &mut next_id);
        remaining = remove_time_range(remaining, target_start, target_end, &mut next_id);
        move_slice_notes(&mut slice, source_start, target_start, &mut next_id);
        remaining.extend(slice);
        sort_notes(&mut remaining);
        self.notes = remaining;
        self.edit_cursor_beat = Some(target_start);
        self.time_selection = Some((target_start, target_end));
        self.replace_selection(std::iter::empty());
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

fn split_notes_for_time_range(
    notes: Vec<PianoNote>,
    start_beat: f32,
    end_beat: f32,
    remove_inner: bool,
    next_id: &mut u32,
) -> (Vec<PianoNote>, Vec<PianoNote>) {
    let mut remaining = Vec::with_capacity(notes.len());
    let mut slice = Vec::new();

    for note in notes {
        if !note_overlaps_range(note, start_beat, end_beat) {
            remaining.push(note);
            continue;
        }

        if !remove_inner {
            remaining.push(note);
        } else {
            push_note_segment(&mut remaining, note, note.start_beat, start_beat, next_id);
            push_note_segment(&mut remaining, note, end_beat, note.end_beat(), next_id);
        }

        push_note_segment(
            &mut slice,
            note,
            note.start_beat.max(start_beat),
            note.end_beat().min(end_beat),
            next_id,
        );
    }

    (remaining, slice)
}

fn remove_time_range(
    notes: Vec<PianoNote>,
    start_beat: f32,
    end_beat: f32,
    next_id: &mut u32,
) -> Vec<PianoNote> {
    let mut remaining = Vec::with_capacity(notes.len());
    for note in notes {
        if !note_overlaps_range(note, start_beat, end_beat) {
            remaining.push(note);
            continue;
        }
        push_note_segment(&mut remaining, note, note.start_beat, start_beat, next_id);
        push_note_segment(&mut remaining, note, end_beat, note.end_beat(), next_id);
    }
    remaining
}

fn move_slice_notes(
    notes: &mut [PianoNote],
    source_start_beat: f32,
    target_start_beat: f32,
    next_id: &mut u32,
) {
    let beat_delta = target_start_beat - source_start_beat;
    for note in notes {
        note.id = allocate_note_id(next_id);
        note.start_beat = (note.start_beat + beat_delta).clamp(0.0, TOTAL_BEATS);
    }
}

fn push_note_segment(
    notes: &mut Vec<PianoNote>,
    source: PianoNote,
    start_beat: f32,
    end_beat: f32,
    next_id: &mut u32,
) {
    let start_beat = start_beat.max(source.start_beat).clamp(0.0, TOTAL_BEATS);
    let end_beat = end_beat.min(source.end_beat()).clamp(0.0, TOTAL_BEATS);
    let length_beats = end_beat - start_beat;
    if length_beats <= f32::EPSILON {
        return;
    }
    notes.push(PianoNote {
        id: allocate_note_id(next_id),
        start_beat,
        length_beats,
        ..source
    });
}

fn note_overlaps_range(note: PianoNote, start_beat: f32, end_beat: f32) -> bool {
    note.start_beat < end_beat && note.end_beat() > start_beat
}

fn allocate_note_id(next_id: &mut u32) -> u32 {
    let id = *next_id;
    *next_id = (*next_id).saturating_add(1);
    id
}

fn sort_notes(notes: &mut [PianoNote]) {
    notes.sort_by(|a, b| {
        a.start_beat
            .total_cmp(&b.start_beat)
            .then_with(|| a.pitch.cmp(&b.pitch))
            .then_with(|| a.id.cmp(&b.id))
    });
}
