//! Bounded transient grouping metadata; applications own durable edit history.

use super::{EditEvent, EditPhase, EditTransaction, InteractionProvenance};

/// The operation that determines continuity between text edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextEditKind {
    /// Ordinary adjacent text insertion.
    Typing,
    /// Deletion toward the preceding text.
    BackwardDelete,
    /// Deletion toward the following text.
    ForwardDelete,
    /// One clipboard replacement or cut.
    Clipboard,
    /// One semantic whole-value replacement.
    Replacement,
    /// One native preedit/commit/cancel session.
    Composition,
}

/// The explicit boundary that ended a transient group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextEditBoundary {
    /// Caret or selection navigation broke edit continuity.
    Selection,
    /// Keyboard focus left the text control.
    FocusLost,
    /// A different kind of edit began.
    KindChanged,
    /// A clipboard operation is atomic.
    Clipboard,
    /// A semantic replacement is atomic.
    Replacement,
    /// The native composition committed or canceled.
    Composition,
}

/// One text-free lifecycle event for an application-owned history group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextEditGroupEvent {
    /// Opaque process-local transaction; never persist this identity.
    pub transaction: EditTransaction,
    /// Kind fixed when the group begins.
    pub kind: TextEditKind,
    /// First edit, continuation, commit boundary, or canceled composition.
    pub phase: EditPhase,
    /// Reason for a terminal event, when present.
    pub boundary: Option<TextEditBoundary>,
}

/// Grouping attached to one typed edit, without retaining a value or history.
///
/// A transition may finish a previous group and start a new group on the same
/// edit. Atomic clipboard edits have a single `Commit` event. Selection/focus
/// boundaries may finish a group without changing the text value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextEditGrouping {
    /// Previous group closed before applying this edit.
    pub ended: Option<TextEditGroupEvent>,
    /// Group receiving this edit, if it is a content/composition edit.
    pub current: Option<TextEditGroupEvent>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct TextEditGroups {
    active: Option<(TextEditKind, EditEvent<()>)>,
}

impl TextEditGroups {
    pub(crate) fn is_active(&self) -> bool {
        self.active.is_some()
    }

    pub(crate) fn boundary(&mut self, boundary: TextEditBoundary) -> TextEditGrouping {
        TextEditGrouping {
            ended: self.active.take().map(|(kind, event)| TextEditGroupEvent {
                transaction: event.transaction,
                kind,
                phase: EditPhase::Commit,
                boundary: Some(boundary),
            }),
            current: None,
        }
    }

    pub(crate) fn edit(&mut self, kind: TextEditKind) -> TextEditGrouping {
        if matches!(kind, TextEditKind::Clipboard | TextEditKind::Replacement) {
            let boundary = if kind == TextEditKind::Clipboard {
                TextEditBoundary::Clipboard
            } else {
                TextEditBoundary::Replacement
            };
            let mut grouping = self.boundary(boundary);
            let event = EditEvent::begin((), InteractionProvenance::Programmatic);
            grouping.current = Some(TextEditGroupEvent {
                transaction: event.transaction,
                kind,
                phase: EditPhase::Commit,
                boundary: Some(boundary),
            });
            return grouping;
        }
        let mut grouping = if self
            .active
            .as_ref()
            .is_some_and(|(active, _)| *active != kind)
        {
            self.boundary(TextEditBoundary::KindChanged)
        } else {
            TextEditGrouping::default()
        };
        let (event, phase) = match self.active.take() {
            Some((_, event)) => (event, EditPhase::Update),
            None => (
                EditEvent::begin((), InteractionProvenance::Keyboard { timestamp: None }),
                EditPhase::Begin,
            ),
        };
        grouping.current = Some(TextEditGroupEvent {
            transaction: event.transaction,
            kind,
            phase,
            boundary: None,
        });
        self.active = Some((kind, event));
        grouping
    }

    pub(crate) fn finish_composition(&mut self, cancel: bool) -> TextEditGrouping {
        let Some((kind, event)) = self.active.take() else {
            return TextEditGrouping::default();
        };
        if kind != TextEditKind::Composition {
            return TextEditGrouping {
                ended: Some(TextEditGroupEvent {
                    transaction: event.transaction,
                    kind,
                    phase: EditPhase::Commit,
                    boundary: Some(TextEditBoundary::Composition),
                }),
                current: None,
            };
        }
        TextEditGrouping {
            ended: None,
            current: Some(TextEditGroupEvent {
                transaction: event.transaction,
                kind,
                phase: if cancel {
                    EditPhase::Cancel
                } else {
                    EditPhase::Commit
                },
                boundary: Some(TextEditBoundary::Composition),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typing_continues_until_selection_and_clipboard_is_atomic() {
        let mut groups = TextEditGroups::default();
        let first = groups.edit(TextEditKind::Typing).current.unwrap();
        let second = groups.edit(TextEditKind::Typing).current.unwrap();
        assert_eq!(first.phase, EditPhase::Begin);
        assert_eq!(second.phase, EditPhase::Update);
        assert_eq!(first.transaction, second.transaction);
        assert_eq!(
            groups
                .boundary(TextEditBoundary::Selection)
                .ended
                .unwrap()
                .transaction,
            first.transaction
        );
        let paste = groups.edit(TextEditKind::Clipboard).current.unwrap();
        assert_eq!(paste.phase, EditPhase::Commit);
        assert!(!groups.is_active());
        assert_ne!(
            groups
                .edit(TextEditKind::Typing)
                .current
                .unwrap()
                .transaction,
            first.transaction
        );
    }
    #[test]
    fn composition_keeps_one_group_and_reports_cancel() {
        let mut groups = TextEditGroups::default();
        let typing = groups.edit(TextEditKind::Typing).current.unwrap();
        let start = groups.edit(TextEditKind::Composition);
        assert_eq!(start.ended.unwrap().transaction, typing.transaction);
        let transaction = start.current.unwrap().transaction;
        assert_eq!(
            groups
                .edit(TextEditKind::Composition)
                .current
                .unwrap()
                .transaction,
            transaction
        );
        let cancel = groups.finish_composition(true).current.unwrap();
        assert_eq!(cancel.transaction, transaction);
        assert_eq!(cancel.phase, EditPhase::Cancel);
        assert!(!groups.is_active());
    }
}
