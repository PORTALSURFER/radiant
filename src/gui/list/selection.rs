/// Modifier state for an index-list selection request.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ListSelectionModifiers {
    /// Extend selection from the current anchor to the requested index.
    pub extend: bool,
    /// Toggle the requested index without replacing the rest of the selection.
    pub toggle: bool,
}

impl ListSelectionModifiers {
    /// Build empty selection modifiers.
    pub const fn new() -> Self {
        Self {
            extend: false,
            toggle: false,
        }
    }

    /// Build modifiers for range extension.
    pub const fn extend() -> Self {
        Self {
            extend: true,
            toggle: false,
        }
    }

    /// Build modifiers for membership toggle.
    pub const fn toggle() -> Self {
        Self {
            extend: false,
            toggle: true,
        }
    }
}

/// Move an item index by a signed delta, clamped to the current list bounds.
///
/// This helper is intentionally stateless so hosts can keep durable selection
/// identity in paths, ids, or other domain keys while sharing the generic
/// keyboard-navigation rule.
pub fn list_index_after_delta(current: usize, delta: isize, total_items: usize) -> Option<usize> {
    if total_items == 0 {
        return None;
    }
    let last_index = total_items.saturating_sub(1);
    if delta.is_negative() {
        Some(current.saturating_sub(delta.unsigned_abs()).min(last_index))
    } else {
        Some(current.saturating_add(delta as usize).min(last_index))
    }
}

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
        self.replace_selection(Vec::new());
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
            self.select_range(anchor, index);
        } else if modifiers.toggle {
            self.anchor_index = Some(index);
            self.toggle_index(index);
        } else {
            self.anchor_index = Some(index);
            self.replace_selection(vec![index]);
        }
        true
    }

    /// Select every row in the current item range.
    pub fn select_all(&mut self, total_items: usize) {
        let selected = (0..total_items).collect::<Vec<_>>();
        if self.focused_index.is_none() && total_items > 0 {
            self.focused_index = Some(0);
        }
        self.anchor_index = self.focused_index;
        self.replace_selection(selected);
    }

    fn select_range(&mut self, anchor: usize, index: usize) {
        let start = anchor.min(index);
        let end = anchor.max(index);
        self.replace_selection((start..=end).collect());
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

    fn replace_selection(&mut self, mut selected: Vec<usize>) {
        selected.sort_unstable();
        selected.dedup();
        if self.selected_indices != selected {
            self.selected_indices = selected;
            self.bump_revision();
        }
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }
}
