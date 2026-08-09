//! Parser-agnostic lifecycle for numeric text editing.

use super::{EditEvent, InteractionProvenance};

/// A parser-agnostic session for editing a typed numeric value as text.
///
/// The session keeps the caller-provided draft verbatim while reusing the
/// shared [`EditEvent`] lifecycle for its terminal boundary. Draft replacement
/// never produces a typed update. The caller owns parsing, validation, domain
/// policy, and any application or widget integration around the session.
#[derive(Clone)]
pub struct NumericEditSession<T> {
    draft: String,
    begin: EditEvent<T>,
}

impl<T: Clone> NumericEditSession<T> {
    /// Begin a session at `value` with the caller-provided `draft` text.
    ///
    /// This creates exactly one [`EditEvent::begin`] event. The draft is kept
    /// as supplied, including empty, incomplete, or invalid text.
    pub fn begin(value: T, draft: impl Into<String>, provenance: InteractionProvenance) -> Self {
        Self {
            draft: draft.into(),
            begin: EditEvent::begin(value, provenance),
        }
    }

    /// Return the current draft text without interpreting it.
    #[must_use]
    pub fn draft(&self) -> &str {
        &self.draft
    }

    /// Replace the draft text verbatim without emitting a typed update.
    pub fn replace_draft(&mut self, draft: impl Into<String>) {
        self.draft = draft.into();
    }

    /// Return the session's initial Begin event.
    #[must_use]
    pub fn begin_event(&self) -> &EditEvent<T> {
        &self.begin
    }

    /// Commit a caller-certified typed value without parsing or validating it.
    ///
    /// A provenance with the same [`InteractionProvenance::source`] category
    /// as the Begin event is accepted even when its native metadata differs.
    /// A foreign source returns the unchanged session in `Err`.
    pub fn commit(
        self,
        accepted_value: T,
        provenance: InteractionProvenance,
    ) -> Result<EditEvent<T>, Self> {
        if provenance.source() != self.begin.transaction.source() {
            return Err(self);
        }

        match self.begin.commit(accepted_value, provenance) {
            Some(event) => Ok(event),
            None => unreachable!("NumericEditSession always retains its Begin event"),
        }
    }

    /// Cancel the session and restore the value captured by its Begin event.
    ///
    /// A provenance with the same source category as the Begin event is
    /// accepted. A foreign source returns the unchanged session in `Err`.
    pub fn cancel(self, provenance: InteractionProvenance) -> Result<EditEvent<T>, Self> {
        if provenance.source() != self.begin.transaction.source() {
            return Err(self);
        }

        match self.begin.cancel(provenance) {
            Some(event) => Ok(event),
            None => unreachable!("NumericEditSession always retains its Begin event"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::input::{InputSequence, InputSequenceRange, InputTimestamp},
        widgets::{EditPhase, PointerModifiers},
    };

    #[derive(Clone, Debug, PartialEq)]
    struct NonF32(u32);

    fn pointer_provenance(
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
        sequence_range: Option<InputSequenceRange>,
    ) -> InteractionProvenance {
        InteractionProvenance::Pointer {
            modifiers,
            timestamp,
            sequence_range,
        }
    }

    #[test]
    fn preserves_verbatim_drafts_without_typed_updates() {
        let provenance = InteractionProvenance::Keyboard { timestamp: None };
        let mut session = NumericEditSession::begin(NonF32(7), "7", provenance);

        for draft in ["", "-", "1e", "not a number"] {
            session.replace_draft(draft);
            assert_eq!(session.draft(), draft);
            assert_eq!(session.begin_event().phase, EditPhase::Begin);
            assert_eq!(session.begin_event().value, NonF32(7));
        }
    }

    #[test]
    fn commits_caller_certified_value_with_same_source_metadata_changes() {
        let begin_provenance = pointer_provenance(
            PointerModifiers::default(),
            Some(InputTimestamp::capture()),
            Some(InputSequenceRange::singleton(
                InputSequence::from_runtime_value(4),
            )),
        );
        let commit_provenance = pointer_provenance(
            PointerModifiers {
                shift: true,
                ..PointerModifiers::default()
            },
            Some(InputTimestamp::capture()),
            Some(InputSequenceRange::singleton(
                InputSequence::from_runtime_value(9),
            )),
        );
        let session = NumericEditSession::begin(NonF32(7), "not a number", begin_provenance);
        let transaction = session.begin_event().transaction;

        let event = match session.commit(NonF32(u32::MAX), commit_provenance) {
            Ok(event) => event,
            Err(_) => panic!("same source should commit"),
        };

        assert_eq!(event.transaction, transaction);
        assert_eq!(event.phase, EditPhase::Commit);
        assert_eq!(event.start_value, NonF32(7));
        assert_eq!(event.value, NonF32(u32::MAX));
        assert_eq!(event.provenance, commit_provenance);
    }

    #[test]
    fn cancellation_restores_start_with_same_source_metadata_changes() {
        let begin_provenance = pointer_provenance(
            PointerModifiers::default(),
            Some(InputTimestamp::capture()),
            None,
        );
        let cancel_provenance = pointer_provenance(
            PointerModifiers {
                command: true,
                ..PointerModifiers::default()
            },
            Some(InputTimestamp::capture()),
            Some(InputSequenceRange::singleton(
                InputSequence::from_runtime_value(12),
            )),
        );
        let session = NumericEditSession::begin(NonF32(7), "-", begin_provenance);
        let transaction = session.begin_event().transaction;

        let event = match session.cancel(cancel_provenance) {
            Ok(event) => event,
            Err(_) => panic!("same source should cancel"),
        };

        assert_eq!(event.transaction, transaction);
        assert_eq!(event.phase, EditPhase::Cancel);
        assert_eq!(event.start_value, NonF32(7));
        assert_eq!(event.value, NonF32(7));
        assert_eq!(event.provenance, cancel_provenance);
    }

    #[test]
    fn foreign_source_preserves_session_for_commit_and_cancel() {
        let pointer = pointer_provenance(PointerModifiers::default(), None, None);
        let keyboard = InteractionProvenance::Keyboard { timestamp: None };
        let session = NumericEditSession::begin(NonF32(7), "1e", pointer);
        let transaction = session.begin_event().transaction;

        let session = match session.commit(NonF32(99), keyboard) {
            Ok(_) => panic!("foreign source must not commit"),
            Err(session) => session,
        };
        assert_eq!(session.draft(), "1e");
        assert_eq!(session.begin_event().transaction, transaction);
        assert_eq!(session.begin_event().phase, EditPhase::Begin);
        assert_eq!(session.begin_event().value, NonF32(7));

        let session = match session.cancel(keyboard) {
            Ok(_) => panic!("foreign source must not cancel"),
            Err(session) => session,
        };
        assert_eq!(session.draft(), "1e");
        assert_eq!(session.begin_event().transaction, transaction);
        assert_eq!(session.begin_event().phase, EditPhase::Begin);
    }

    #[test]
    fn successful_terminal_transitions_consume_the_session() {
        let provenance = InteractionProvenance::Programmatic;
        let commit = match NumericEditSession::begin(NonF32(1), "1", provenance)
            .commit(NonF32(2), provenance)
        {
            Ok(event) => event,
            Err(_) => panic!("matching commit should succeed"),
        };
        assert!(commit.phase.is_terminal());

        let cancel = match NumericEditSession::begin(NonF32(3), "3", provenance).cancel(provenance)
        {
            Ok(event) => event,
            Err(_) => panic!("matching cancel should succeed"),
        };
        assert!(cancel.phase.is_terminal());
    }
}
