use super::{ListSelectionIntent, ListSelectionModifiers};

/// Reusable index-based focus, anchor, and multi-selection state for dense lists.
///
/// Hosts keep ownership of durable row identity. This type tracks logical row
/// indices so apps can map selected rows back to paths, database ids, or other
/// domain keys after filtering and sorting.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListSelectionController {
    focused_index: Option<usize>,
    anchor_index: Option<usize>,
    selected_indices: Vec<usize>,
    revision: u64,
}

impl ListSelectionController {
    /// Build an empty selection controller.
    pub const fn new() -> Self {
        Self {
            focused_index: None,
            anchor_index: None,
            selected_indices: Vec::new(),
            revision: 0,
        }
    }

    /// Return the focused row index, if any.
    pub const fn focused_index(&self) -> Option<usize> {
        self.focused_index
    }

    /// Return the range-selection anchor index, if any.
    pub const fn anchor_index(&self) -> Option<usize> {
        self.anchor_index
    }

    /// Return sorted selected row indices.
    pub fn selected_indices(&self) -> &[usize] {
        &self.selected_indices
    }

    /// Return a monotonic revision bumped when selection membership changes.
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Return whether the requested row index is currently selected.
    pub fn is_selected(&self, index: usize) -> bool {
        self.selected_indices.binary_search(&index).is_ok()
    }

    /// Clear focus, anchor, and selected membership.
    pub fn clear(&mut self) {
        self.focused_index = None;
        self.anchor_index = None;
        self.replace_empty_selection();
    }

    /// Clamp focus, anchor, and selected membership to the current item count.
    pub fn clamp_to_len(&mut self, total_items: usize) {
        if self.focused_index.is_some_and(|index| index >= total_items) {
            self.focused_index = None;
        }
        if self.anchor_index.is_some_and(|index| index >= total_items) {
            self.anchor_index = self.focused_index;
        }
        let original_len = self.selected_indices.len();
        self.selected_indices.retain(|index| *index < total_items);
        if self.selected_indices.len() != original_len {
            self.bump_revision();
        }
    }

    /// Move focus without changing selection membership.
    pub fn focus(&mut self, index: usize, total_items: usize) -> bool {
        if index >= total_items {
            return false;
        }
        self.focused_index = Some(index);
        self.anchor_index.get_or_insert(index);
        true
    }

    /// Apply a pointer or keyboard selection request for one row.
    pub fn select(
        &mut self,
        index: usize,
        total_items: usize,
        modifiers: ListSelectionModifiers,
    ) -> bool {
        if index >= total_items {
            return false;
        }
        self.focused_index = Some(index);
        if modifiers.extend {
            let anchor = self.anchor_index.unwrap_or(index).min(total_items - 1);
            self.anchor_index = Some(anchor);
            self.select_range(anchor, index, false);
        } else if modifiers.toggle {
            self.anchor_index = Some(index);
            self.toggle_index(index);
        } else {
            self.anchor_index = Some(index);
            self.replace_single_selection(index);
        }
        true
    }

    /// Apply a high-level pointer or keyboard selection intent for one row.
    pub fn select_with_intent(
        &mut self,
        index: usize,
        total_items: usize,
        intent: ListSelectionIntent,
    ) -> bool {
        match intent {
            ListSelectionIntent::ExtendPreservingExisting => {
                self.extend_preserving_existing(index, total_items)
            }
            _ => self.select(index, total_items, intent.modifiers()),
        }
    }

    /// Extend selection from the current anchor while preserving existing membership.
    pub fn extend_preserving_existing(&mut self, index: usize, total_items: usize) -> bool {
        if index >= total_items {
            return false;
        }
        self.focused_index = Some(index);
        let anchor = self.anchor_index.unwrap_or(index).min(total_items - 1);
        self.anchor_index = Some(anchor);
        self.select_range(anchor, index, true);
        true
    }

    /// Select every row in the current item range.
    pub fn select_all(&mut self, total_items: usize) {
        if self.focused_index.is_none() && total_items > 0 {
            self.focused_index = Some(0);
        }
        self.anchor_index = self.focused_index;
        self.replace_selection_range(0, total_items);
    }

    fn select_range(&mut self, anchor: usize, index: usize, preserve_existing: bool) {
        let start = anchor.min(index);
        let end = anchor.max(index);
        if preserve_existing {
            self.extend_selection_range(start, end.saturating_add(1));
        } else {
            self.replace_selection_range(start, end.saturating_add(1));
        }
    }

    fn toggle_index(&mut self, index: usize) {
        match self.selected_indices.binary_search(&index) {
            Ok(position) => {
                self.selected_indices.remove(position);
                self.bump_revision();
            }
            Err(position) => {
                self.selected_indices.insert(position, index);
                self.bump_revision();
            }
        }
    }

    fn replace_empty_selection(&mut self) {
        if !self.selected_indices.is_empty() {
            self.selected_indices.clear();
            self.bump_revision();
        }
    }

    fn replace_single_selection(&mut self, index: usize) {
        if self.selected_indices.len() == 1 && self.selected_indices[0] == index {
            return;
        }
        self.selected_indices.clear();
        self.selected_indices.push(index);
        self.bump_revision();
    }

    fn replace_selection_range(&mut self, start: usize, end_exclusive: usize) {
        if self
            .selected_indices
            .iter()
            .copied()
            .eq(start..end_exclusive)
        {
            return;
        }
        self.selected_indices.clear();
        self.selected_indices.extend(start..end_exclusive);
        self.bump_revision();
    }

    fn extend_selection_range(&mut self, start: usize, end_exclusive: usize) {
        if start >= end_exclusive {
            return;
        }

        let replace_start = self
            .selected_indices
            .partition_point(|index| *index < start);
        let replace_end = self
            .selected_indices
            .partition_point(|index| *index < end_exclusive);
        if self.selected_indices[replace_start..replace_end]
            .iter()
            .copied()
            .eq(start..end_exclusive)
        {
            return;
        }

        self.selected_indices
            .splice(replace_start..replace_end, start..end_exclusive);
        self.bump_revision();
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }
}
