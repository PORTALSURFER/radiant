use crate::runtime::ExactChangedRoot;
use std::rc::Rc;

/// One immutable cache result and, when proven, its immediate predecessor.
/// Only the predecessor's token is retained, never a chain of old subtrees.
pub(crate) struct ComponentSnapshot {
    identity: Rc<()>,
    predecessor: Option<Rc<()>>,
    changes: Box<[ExactChangedRoot]>,
}

impl ComponentSnapshot {
    pub(super) fn new(previous: Option<&Self>, changes: Option<Vec<ExactChangedRoot>>) -> Self {
        let predecessor = changes
            .as_ref()
            .and(previous)
            .map(|old| Rc::clone(&old.identity));
        Self {
            identity: Rc::new(()),
            predecessor,
            changes: changes.unwrap_or_default().into_boxed_slice(),
        }
    }

    /// Only a direct, validated transition from the committed snapshot admits
    /// changed descendants. Skipped or aborted projections use full refresh.
    pub(crate) fn changes_since(&self, previous: &Self) -> Option<&[ExactChangedRoot]> {
        if Rc::ptr_eq(&self.identity, &previous.identity) {
            Some(&[])
        } else if self
            .predecessor
            .as_ref()
            .is_some_and(|old| Rc::ptr_eq(old, &previous.identity))
        {
            Some(&self.changes)
        } else {
            None
        }
    }
}
