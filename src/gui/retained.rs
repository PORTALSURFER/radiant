//! Retained snapshot storage primitives shared by runtime-facing models.

use std::{ops::Deref, sync::Arc};

/// Shared vector storage used by retained immutable snapshots.
///
/// Runtime bridges often clone top-level model snapshots while large segment
/// payloads stay unchanged. Keeping vectors behind a shared container makes
/// those clones cheap while still allowing the active snapshot to mutate
/// content through `Arc::make_mut` when a retained window changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetainedVec<T>(Arc<Vec<T>>);

impl<T> RetainedVec<T> {
    /// Build an empty retained vector.
    pub fn new() -> Self {
        Self(Arc::new(Vec::new()))
    }

    /// Append one element, cloning the backing vector only when aliased.
    pub fn push(&mut self, value: T)
    where
        T: Clone,
    {
        Arc::make_mut(&mut self.0).push(value);
    }

    /// Clear all elements, preserving retained storage when possible.
    pub fn clear(&mut self)
    where
        T: Clone,
    {
        Arc::make_mut(&mut self.0).clear();
    }

    /// Truncate the vector to `len`.
    pub fn truncate(&mut self, len: usize)
    where
        T: Clone,
    {
        Arc::make_mut(&mut self.0).truncate(len);
    }

    /// Return the number of retained elements.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Return whether the retained vector is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Borrow the retained contents as a slice.
    pub fn as_slice(&self) -> &[T] {
        self.0.as_slice()
    }

    /// Borrow one retained element by index.
    pub fn get(&self, index: usize) -> Option<&T> {
        self.0.get(index)
    }

    /// Borrow one retained element mutably, cloning the backing vector only when aliased.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T>
    where
        T: Clone,
    {
        Arc::make_mut(&mut self.0).get_mut(index)
    }

    /// Borrow the backing vector mutably for batched updates.
    pub fn make_mut(&mut self) -> &mut Vec<T>
    where
        T: Clone,
    {
        Arc::make_mut(&mut self.0)
    }
}

impl<T> Default for RetainedVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Deref for RetainedVec<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T> From<Vec<T>> for RetainedVec<T> {
    fn from(value: Vec<T>) -> Self {
        Self(Arc::new(value))
    }
}

#[cfg(test)]
mod tests {
    use super::RetainedVec;

    #[test]
    fn retained_vec_clones_share_storage_until_mutation() {
        let mut original = RetainedVec::from(vec![1, 2, 3]);
        let clone = original.clone();

        original.push(4);

        assert_eq!(clone.as_slice(), &[1, 2, 3]);
        assert_eq!(original.as_slice(), &[1, 2, 3, 4]);
    }
}
