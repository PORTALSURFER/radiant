use crate::gui::selection::SelectionSet;

use super::key_membership::{OrderedKeyMembership, ordered_key_index};
use super::{ListSelectionIntent, ListSelectionModifiers, list_index_after_delta};

/// Reusable focus, anchor, and multi-selection state for lists keyed by stable row identity.
///
/// Hosts pass the current ordered visible keys into operations that need list
/// order, while this controller keeps durable selection identity in generic
/// keys such as paths, database ids, document ids, or item keys.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyedListSelection<K> {
    focused_key: Option<K>,
    anchor_key: Option<K>,
    selected_keys: SelectionSet<K>,
    revision: u64,
}

impl<K> KeyedListSelection<K> {
    /// Build an empty keyed list selection.
    pub const fn new() -> Self {
        Self {
            focused_key: None,
            anchor_key: None,
            selected_keys: SelectionSet::new(),
            revision: 0,
        }
    }

    /// Return the focused row key, if any.
    pub fn focused_key(&self) -> Option<&K> {
        self.focused_key.as_ref()
    }

    /// Return the range-selection anchor key, if any.
    pub fn anchor_key(&self) -> Option<&K> {
        self.anchor_key.as_ref()
    }

    /// Return sorted selected row keys.
    pub fn selected_keys(&self) -> &[K] {
        self.selected_keys.as_slice()
    }

    /// Return the selected row count.
    pub fn selected_count(&self) -> usize {
        self.selected_keys.len()
    }

    /// Return whether the keyed selection is empty.
    pub fn is_empty(&self) -> bool {
        self.selected_keys.is_empty()
    }

    /// Return a monotonic revision bumped when selection membership changes.
    pub const fn revision(&self) -> u64 {
        self.revision
    }
}

impl<K> KeyedListSelection<K>
where
    K: Ord,
{
    /// Return whether `key` is currently selected.
    pub fn is_selected(&self, key: &K) -> bool {
        self.selected_keys.contains(key)
    }
}

impl<K> KeyedListSelection<K>
where
    K: Clone + Ord,
{
    /// Build a keyed list selection from host-owned selection parts.
    pub fn from_parts(
        focused_key: Option<K>,
        anchor_key: Option<K>,
        selected_keys: impl IntoIterator<Item = K>,
    ) -> Self {
        Self {
            focused_key,
            anchor_key,
            selected_keys: SelectionSet::from_items(selected_keys),
            revision: 0,
        }
    }

    /// Clear focus, anchor, and selected membership.
    pub fn clear(&mut self) {
        self.focused_key = None;
        self.anchor_key = None;
        if self.selected_keys.clear() {
            self.bump_revision();
        }
    }

    /// Remove selected keys that are not present in `ordered_keys`.
    pub fn retain_visible(&mut self, ordered_keys: &[K]) {
        let membership =
            OrderedKeyMembership::new(ordered_keys, self.selected_keys.len().saturating_add(2));
        if self
            .selected_keys
            .retain_items(|key| membership.contains(key))
        {
            self.bump_revision();
        }
        if self
            .focused_key
            .as_ref()
            .is_some_and(|key| !membership.contains(key))
        {
            self.focused_key = None;
        }
        if self
            .anchor_key
            .as_ref()
            .is_some_and(|key| !membership.contains(key))
        {
            self.anchor_key = self.focused_key.clone();
        }
    }

    /// Move focus without changing selection membership.
    pub fn focus(&mut self, key: K, ordered_keys: &[K]) -> bool {
        if ordered_key_index(&key, ordered_keys).is_none() {
            return false;
        }
        self.focused_key = Some(key.clone());
        self.anchor_key.get_or_insert(key);
        true
    }

    /// Move focus by `delta`, optionally extending selection.
    pub fn navigate(&mut self, delta: isize, ordered_keys: &[K], extend: bool) -> Option<K> {
        self.navigate_with(delta, ordered_keys, extend, false)
    }

    /// Move focus by `delta`, extending selection while preserving existing membership.
    pub fn navigate_preserving_existing(&mut self, delta: isize, ordered_keys: &[K]) -> Option<K> {
        self.navigate_with(delta, ordered_keys, true, true)
    }

    fn navigate_with(
        &mut self,
        delta: isize,
        ordered_keys: &[K],
        extend: bool,
        preserve_existing: bool,
    ) -> Option<K> {
        let current = self.focused_key.as_ref()?;
        let current_index = ordered_key_index(current, ordered_keys)?;
        let target_index = list_index_after_delta(current_index, delta, ordered_keys.len())?;
        if target_index == current_index {
            return None;
        }
        let target = ordered_keys[target_index].clone();
        let modifiers = if extend {
            ListSelectionModifiers::extend()
        } else {
            ListSelectionModifiers::new()
        };
        if preserve_existing {
            self.extend_preserving_existing_at_index(target.clone(), target_index, ordered_keys);
        } else {
            self.select_at_index(target.clone(), target_index, ordered_keys, modifiers);
        }
        Some(target)
    }

    /// Apply a pointer or keyboard selection request for one keyed row.
    pub fn select(
        &mut self,
        key: K,
        ordered_keys: &[K],
        modifiers: ListSelectionModifiers,
    ) -> bool {
        let Some(key_index) = ordered_key_index(&key, ordered_keys) else {
            return false;
        };
        self.select_at_index(key, key_index, ordered_keys, modifiers);
        true
    }

    fn select_at_index(
        &mut self,
        key: K,
        key_index: usize,
        ordered_keys: &[K],
        modifiers: ListSelectionModifiers,
    ) {
        self.focused_key = Some(key.clone());
        if modifiers.extend {
            let anchor_index = self
                .anchor_key
                .as_ref()
                .and_then(|anchor| ordered_key_index(anchor, ordered_keys))
                .unwrap_or(key_index);
            self.anchor_key = Some(ordered_keys[anchor_index].clone());
            self.select_range_indices(anchor_index, key_index, ordered_keys, false);
        } else if modifiers.toggle {
            self.anchor_key = Some(key.clone());
            self.toggle_key(key);
        } else {
            self.anchor_key = Some(key.clone());
            self.replace_selection([key]);
        }
    }

    /// Apply a high-level pointer or keyboard selection intent for one keyed row.
    pub fn select_with_intent(
        &mut self,
        key: K,
        ordered_keys: &[K],
        intent: ListSelectionIntent,
    ) -> bool {
        match intent {
            ListSelectionIntent::ExtendPreservingExisting => {
                self.extend_preserving_existing(key, ordered_keys)
            }
            _ => self.select(key, ordered_keys, intent.modifiers()),
        }
    }

    /// Extend selection from the current anchor while preserving existing membership.
    pub fn extend_preserving_existing(&mut self, key: K, ordered_keys: &[K]) -> bool {
        let Some(key_index) = ordered_key_index(&key, ordered_keys) else {
            return false;
        };
        self.extend_preserving_existing_at_index(key, key_index, ordered_keys);
        true
    }

    fn extend_preserving_existing_at_index(
        &mut self,
        key: K,
        key_index: usize,
        ordered_keys: &[K],
    ) {
        self.focused_key = Some(key.clone());
        let anchor_index = self
            .anchor_key
            .as_ref()
            .and_then(|anchor| ordered_key_index(anchor, ordered_keys))
            .unwrap_or(key_index);
        self.anchor_key = Some(ordered_keys[anchor_index].clone());
        self.select_range_indices(anchor_index, key_index, ordered_keys, true);
    }

    /// Select every key in the current ordered list.
    pub fn select_all(&mut self, ordered_keys: &[K]) {
        if self.focused_key.is_none()
            && let Some(first) = ordered_keys.first()
        {
            self.focused_key = Some(first.clone());
        }
        self.anchor_key = self.focused_key.clone();
        self.replace_selection(ordered_keys.iter().cloned());
    }

    fn select_range_indices(
        &mut self,
        anchor_index: usize,
        key_index: usize,
        ordered_keys: &[K],
        preserve_existing: bool,
    ) {
        let start = anchor_index.min(key_index);
        let end = anchor_index.max(key_index);
        let range = ordered_keys[start..=end].iter().cloned();
        if preserve_existing {
            if self.selected_keys.extend_items(range) {
                self.bump_revision();
            }
        } else {
            self.replace_selection(range);
        }
    }

    fn toggle_key(&mut self, key: K) {
        let changed = if self.selected_keys.contains(&key) && self.selected_keys.len() > 1 {
            self.selected_keys.remove(&key)
        } else {
            self.selected_keys.insert(key)
        };
        if changed {
            self.bump_revision();
        }
    }

    fn replace_selection(&mut self, selected: impl IntoIterator<Item = K>) {
        if self.selected_keys.replace_items(selected) {
            self.bump_revision();
        }
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }
}
