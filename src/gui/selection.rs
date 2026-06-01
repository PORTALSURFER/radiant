//! Generic selection state primitives.

use serde::{Deserialize, Serialize};

#[cfg(test)]
#[path = "selection/tests.rs"]
mod tests;

/// Three-way state for controls representing multiple selected items.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TriState {
    /// No selected items currently carry the represented value.
    #[default]
    Off,
    /// Every selected item currently carries the represented value.
    On,
    /// Selected items disagree about the represented value.
    Mixed,
}

/// Generic target for three-way selection or triage actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriageTarget {
    /// Move the selection toward the negative/left bucket.
    Negative,
    /// Move the selection toward the neutral/default bucket.
    Neutral,
    /// Move the selection toward the positive/right bucket.
    Positive,
}

/// Sorted unique membership set for dense UI selections and drag groups.
///
/// The set keeps membership checks logarithmic while preserving a compact
/// `Vec` representation that application state can snapshot, diff, serialize,
/// or send through typed widget messages.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectionSet<T> {
    items: Vec<T>,
}

impl<T> SelectionSet<T> {
    /// Build an empty set.
    pub const fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Return the normalized selected items.
    pub fn as_slice(&self) -> &[T] {
        &self.items
    }

    /// Consume the set and return its sorted unique items.
    pub fn into_vec(self) -> Vec<T> {
        self.items
    }

    /// Return the selected item count.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Return whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Return whether `items` are strictly sorted and unique by a projected key.
    pub fn slice_is_sorted_unique_by_key<K>(items: &[T], mut key: impl FnMut(&T) -> K) -> bool
    where
        K: Ord,
    {
        items.windows(2).all(|pair| key(&pair[0]) < key(&pair[1]))
    }
}

impl<T> SelectionSet<T>
where
    T: Ord,
{
    /// Build a set from unsorted and possibly duplicated items.
    pub fn from_items(items: impl IntoIterator<Item = T>) -> Self {
        let mut items = items.into_iter().collect::<Vec<_>>();
        Self::normalize_vec(&mut items);
        Self { items }
    }

    /// Replace the set with normalized `items`.
    pub fn replace_items(&mut self, items: impl IntoIterator<Item = T>) -> bool {
        let mut items = items.into_iter().collect::<Vec<_>>();
        Self::normalize_vec(&mut items);
        if self.items == items {
            return false;
        }
        self.items = items;
        true
    }

    /// Insert one item and return whether membership changed.
    pub fn insert(&mut self, item: T) -> bool {
        match self.items.binary_search(&item) {
            Ok(_) => false,
            Err(position) => {
                self.items.insert(position, item);
                true
            }
        }
    }

    /// Remove one item and return whether membership changed.
    pub fn remove(&mut self, item: &T) -> bool {
        match self.items.binary_search(item) {
            Ok(position) => {
                self.items.remove(position);
                true
            }
            Err(_) => false,
        }
    }

    /// Clear the set and return whether membership changed.
    pub fn clear(&mut self) -> bool {
        let changed = !self.items.is_empty();
        self.items.clear();
        changed
    }

    /// Extend the set with normalized `items`.
    pub fn extend_items(&mut self, items: impl IntoIterator<Item = T>) -> bool {
        let previous_len = self.items.len();
        self.items.extend(items);
        Self::normalize_vec(&mut self.items);
        self.items.len() != previous_len
    }

    /// Return whether the set contains `item`.
    pub fn contains(&self, item: &T) -> bool {
        Self::slice_contains(&self.items, item)
    }

    /// Return whether a sorted unique slice contains `item`.
    pub fn slice_contains(items: &[T], item: &T) -> bool {
        items.binary_search(item).is_ok()
    }

    /// Normalize a vector into sorted unique set order.
    pub fn normalize_vec(items: &mut Vec<T>) {
        items.sort_unstable();
        items.dedup();
    }

    /// Return whether `items` are strictly sorted and unique.
    pub fn slice_is_sorted_unique(items: &[T]) -> bool {
        items.windows(2).all(|pair| pair[0] < pair[1])
    }
}
