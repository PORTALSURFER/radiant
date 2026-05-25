use super::PianoRollState;
use crate::piano_roll::NoteSelectionMode;

impl PianoRollState {
    pub(super) fn delete_selected(&mut self) {
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

    pub(super) fn replace_selection(&mut self, ids: impl IntoIterator<Item = u32>) {
        self.selected_notes = ids
            .into_iter()
            .filter(|id| self.notes.iter().any(|note| note.id == *id))
            .collect();
        self.normalize_selection();
    }

    pub(super) fn select_notes(&mut self, ids: Vec<u32>, mode: NoteSelectionMode) {
        match mode {
            NoteSelectionMode::Replace => self.replace_selection(ids),
            NoteSelectionMode::Add => self.add_notes_to_selection(ids),
            NoteSelectionMode::Toggle => self.toggle_notes_in_selection(ids),
        }
    }

    fn add_notes_to_selection(&mut self, ids: Vec<u32>) {
        self.selected_notes.extend(ids);
        self.selected_notes
            .retain(|id| self.notes.iter().any(|note| note.id == *id));
        self.normalize_selection();
    }

    fn toggle_notes_in_selection(&mut self, ids: Vec<u32>) {
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
        self.normalize_selection();
    }

    fn normalize_selection(&mut self) {
        self.selected_notes.sort_unstable();
        self.selected_notes.dedup();
        self.selected_note = self.selected_notes.first().copied();
    }
}
