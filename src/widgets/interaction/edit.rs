//! Shared lifecycle for continuous widget edits.

use std::sync::atomic::{AtomicU64, Ordering};

use super::{InteractionProvenance, InteractionSource};

static NEXT_EDIT_TRANSACTION_ID: AtomicU64 = AtomicU64::new(1);

/// Lifecycle phase for one continuous widget edit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EditPhase {
    /// The edit has begun at its starting value.
    Begin,
    /// The edit has produced an intermediate value.
    Update,
    /// The edit has completed at its final value.
    Commit,
    /// The edit has been abandoned and should restore its starting value.
    Cancel,
}

impl EditPhase {
    /// Return whether this phase ends its edit transaction.
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Commit | Self::Cancel)
    }
}

/// Opaque process-local identity for one edit transaction.
///
/// A transaction's identity is allocated when its [`EditEvent`] begins. It is
/// meaningful only within the current process and must not be persisted,
/// ordered, serialized, or interpreted as a timestamp. Its source is fixed at
/// the beginning of the transaction and remains stable for every later phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EditTransaction {
    id: u64,
    source: InteractionSource,
}

impl EditTransaction {
    fn new(source: InteractionSource) -> Self {
        Self {
            id: NEXT_EDIT_TRANSACTION_ID.fetch_add(1, Ordering::Relaxed),
            source,
        }
    }

    /// Return the source category selected when this transaction began.
    pub const fn source(self) -> InteractionSource {
        self.source
    }
}

/// One phase of a shared continuous-edit lifecycle.
///
/// The public fields are readable so hosts can reduce the event into their own
/// state and history models. Construct events with [`EditEvent::begin`] and
/// advance them with the bounded lifecycle helpers; an invalid transition
/// returns `None`.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EditEvent<T> {
    /// Opaque identity shared by every phase of this edit.
    pub transaction: EditTransaction,
    /// Lifecycle phase represented by this event.
    pub phase: EditPhase,
    /// Value captured when the transaction began.
    pub start_value: T,
    /// Value carried by this phase. Cancellation restores this to `start_value`.
    pub value: T,
    /// Full observational provenance for the input that produced this phase.
    pub provenance: InteractionProvenance,
}

impl<T: Clone> EditEvent<T> {
    /// Begin an edit at `start_value` using the given interaction provenance.
    ///
    /// The transaction source is selected from `provenance` exactly once here.
    /// The starting and current values of the returned `Begin` event are equal.
    pub fn begin(start_value: T, provenance: InteractionProvenance) -> Self {
        Self {
            transaction: EditTransaction::new(provenance.source()),
            phase: EditPhase::Begin,
            value: start_value.clone(),
            start_value,
            provenance,
        }
    }

    /// Cancel an active edit and restore its starting value.
    ///
    /// Cancellation is accepted only from `Begin` or `Update`, and only when
    /// `provenance` has the same source category as the transaction. The
    /// returned event is a terminal `Cancel` boundary.
    pub fn cancel(self, provenance: InteractionProvenance) -> Option<Self> {
        if !self.can_transition(provenance) {
            return None;
        }

        let value = self.start_value.clone();
        Some(Self {
            transaction: self.transaction,
            phase: EditPhase::Cancel,
            start_value: self.start_value,
            value,
            provenance,
        })
    }
}

impl<T> EditEvent<T> {
    /// Produce an intermediate value for an active edit.
    ///
    /// The transaction and starting value are preserved. The transition is
    /// rejected when the predecessor is terminal or the provenance source
    /// differs from the transaction's fixed source.
    pub fn update(self, value: T, provenance: InteractionProvenance) -> Option<Self> {
        self.transition(EditPhase::Update, value, provenance)
    }

    /// Commit an active edit at `value`.
    ///
    /// The transaction and starting value are preserved. The transition is
    /// rejected when the predecessor is terminal or the provenance source
    /// differs from the transaction's fixed source.
    pub fn commit(self, value: T, provenance: InteractionProvenance) -> Option<Self> {
        self.transition(EditPhase::Commit, value, provenance)
    }

    fn can_transition(&self, provenance: InteractionProvenance) -> bool {
        matches!(self.phase, EditPhase::Begin | EditPhase::Update)
            && provenance.source() == self.transaction.source()
    }

    fn transition(
        self,
        phase: EditPhase,
        value: T,
        provenance: InteractionProvenance,
    ) -> Option<Self> {
        self.can_transition(provenance).then_some(Self {
            transaction: self.transaction,
            phase,
            start_value: self.start_value,
            value,
            provenance,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::PointerModifiers;
    use std::collections::hash_map::DefaultHasher;
    use std::fmt::Debug;
    use std::hash::{Hash, Hasher};

    fn pointer_provenance(modifiers: PointerModifiers) -> InteractionProvenance {
        InteractionProvenance::Pointer {
            modifiers,
            timestamp: None,
            sequence_range: None,
        }
    }

    fn hash<T: Hash>(value: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn edit_phases_mark_only_commit_and_cancel_terminal() {
        assert!(!EditPhase::Begin.is_terminal());
        assert!(!EditPhase::Update.is_terminal());
        assert!(EditPhase::Commit.is_terminal());
        assert!(EditPhase::Cancel.is_terminal());
    }

    #[test]
    fn edit_types_provide_the_required_copy_and_identity_traits() {
        fn assert_phase<T: Clone + Copy + Debug + PartialEq + Eq + Hash>() {}
        fn assert_transaction<T: Clone + Copy + Debug + PartialEq + Eq + Hash>() {}
        fn assert_event<T: Clone + Copy + Debug + PartialEq>() {}

        assert_phase::<EditPhase>();
        assert_transaction::<EditTransaction>();
        assert_event::<EditEvent<f32>>();
    }

    #[test]
    fn begins_are_distinct_and_copied_transactions_keep_identity() {
        let provenance = InteractionProvenance::Keyboard { timestamp: None };
        let first = EditEvent::begin(0.25_f32, provenance);
        let second = EditEvent::begin(0.25_f32, provenance);
        let copied = first.transaction;

        assert_ne!(first.transaction, second.transaction);
        assert_eq!(copied, first.transaction);
        assert_eq!(hash(&copied), hash(&first.transaction));
        assert_eq!(first.phase, EditPhase::Begin);
        assert_eq!(first.start_value, first.value);
        assert_eq!(first.transaction.source(), InteractionSource::Keyboard);
    }

    #[test]
    fn lifecycle_preserves_identity_and_start_while_metadata_changes() {
        let begin_provenance = pointer_provenance(PointerModifiers::default());
        let update_provenance = pointer_provenance(PointerModifiers {
            shift: true,
            ..PointerModifiers::default()
        });
        let commit_provenance = pointer_provenance(PointerModifiers {
            command: true,
            ..PointerModifiers::default()
        });
        let begin = EditEvent::begin(0.25_f32, begin_provenance);
        let transaction = begin.transaction;

        let update = begin
            .update(0.5, update_provenance)
            .expect("Begin should transition to Update");
        assert_eq!(update.transaction, transaction);
        assert_eq!(update.start_value, 0.25);
        assert_eq!(update.value, 0.5);
        assert_eq!(update.provenance, update_provenance);
        assert_eq!(update.transaction.source(), InteractionSource::Pointer);

        let commit = update
            .commit(0.75, commit_provenance)
            .expect("Update should transition to Commit");
        assert_eq!(commit.transaction, transaction);
        assert_eq!(commit.phase, EditPhase::Commit);
        assert_eq!(commit.start_value, 0.25);
        assert_eq!(commit.value, 0.75);
        assert_eq!(commit.provenance, commit_provenance);
    }

    #[test]
    fn cancellation_restores_starting_value_from_begin_or_update() {
        let provenance = InteractionProvenance::Accessibility;
        let begin = EditEvent::begin(3.5_f32, provenance);
        let begin_cancel = begin
            .cancel(provenance)
            .expect("Begin should transition to Cancel");
        assert_eq!(begin_cancel.phase, EditPhase::Cancel);
        assert_eq!(begin_cancel.start_value, 3.5);
        assert_eq!(begin_cancel.value, 3.5);

        let update = EditEvent::begin(3.5_f32, provenance)
            .update(8.0, provenance)
            .expect("Begin should transition to Update");
        let update_cancel = update.cancel(provenance).expect("Update should cancel");
        assert_eq!(update_cancel.phase, EditPhase::Cancel);
        assert_eq!(update_cancel.start_value, 3.5);
        assert_eq!(update_cancel.value, 3.5);
    }

    #[test]
    fn source_mismatch_and_terminal_predecessors_are_rejected() {
        let pointer = pointer_provenance(PointerModifiers::default());
        let keyboard = InteractionProvenance::Keyboard { timestamp: None };
        let begin = EditEvent::begin(0.0_f32, pointer);

        assert!(begin.update(0.5, keyboard).is_none());
        assert!(begin.commit(0.5, keyboard).is_none());
        assert!(begin.cancel(keyboard).is_none());

        let update = begin
            .update(0.5, pointer)
            .expect("matching source should transition");
        let commit = update
            .commit(0.75, pointer)
            .expect("Update should transition to Commit");
        assert!(commit.update(1.0, pointer).is_none());
        assert!(commit.commit(1.0, pointer).is_none());
        assert!(commit.cancel(pointer).is_none());

        let cancel = EditEvent::begin(0.0_f32, pointer)
            .cancel(pointer)
            .expect("Begin should transition to Cancel");
        assert!(cancel.update(1.0, pointer).is_none());
        assert!(cancel.commit(1.0, pointer).is_none());
        assert!(cancel.cancel(pointer).is_none());
    }

    #[test]
    fn every_source_category_is_explicit_even_without_native_metadata() {
        let provenances = [
            (
                pointer_provenance(PointerModifiers::default()),
                InteractionSource::Pointer,
            ),
            (
                InteractionProvenance::Keyboard { timestamp: None },
                InteractionSource::Keyboard,
            ),
            (
                InteractionProvenance::Accessibility,
                InteractionSource::Accessibility,
            ),
            (
                InteractionProvenance::Programmatic,
                InteractionSource::Programmatic,
            ),
        ];

        for (provenance, source) in provenances {
            let event = EditEvent::begin(0.0_f32, provenance);
            assert_eq!(event.provenance.source(), source);
            assert_eq!(event.transaction.source(), source);
        }

        assert_ne!(
            InteractionProvenance::Keyboard { timestamp: None }.source(),
            InteractionSource::Programmatic
        );
    }
}
