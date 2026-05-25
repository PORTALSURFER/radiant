use super::PianoRollState;
use crate::piano_roll::NoteSelectionMode;
use radiant::prelude::SelectionSet;

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
        let note_ids = self.sorted_note_ids();
        let mut next = ids
            .into_iter()
            .filter(|id| note_ids.contains(id))
            .collect::<Vec<_>>();
        SelectionSet::normalize_vec(&mut next);
        if self.selected_notes == next {
            self.selected_note = self.selected_notes.first().copied();
            return;
        }
        self.selected_notes = next;
        self.selected_note = self.selected_notes.first().copied();
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
        let note_ids = self.sorted_note_ids();
        self.selected_notes.retain(|id| note_ids.contains(id));
        self.normalize_selection();
    }

    fn toggle_notes_in_selection(&mut self, ids: Vec<u32>) {
        let note_ids = self.sorted_note_ids();
        for id in ids {
            if let Some(index) = self
                .selected_notes
                .iter()
                .position(|selected| *selected == id)
            {
                self.selected_notes.remove(index);
            } else if note_ids.contains(&id) {
                self.selected_notes.push(id);
            }
        }
        self.normalize_selection();
    }

    fn normalize_selection(&mut self) {
        SelectionSet::normalize_vec(&mut self.selected_notes);
        self.selected_note = self.selected_notes.first().copied();
    }

    fn sorted_note_ids(&self) -> SelectionSet<u32> {
        SelectionSet::from_items(self.notes.iter().map(|note| note.id))
    }
}
