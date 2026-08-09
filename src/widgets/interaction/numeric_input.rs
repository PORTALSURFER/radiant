use super::{EditEvent, EditPhase, EditTransaction};

/// A typed construction failure for the generic text-first numeric input.
///
/// Construction never invents a fallback draft or range. The application must
/// supply both a codec that can format the initial value and an adjustment that
/// can validate its inverse mapping.
#[derive(Debug, PartialEq)]
pub enum NumericInputConstructionError<CodecError, AdjustmentError> {
    /// The codec could not produce the canonical editable draft.
    CodecFormat {
        /// Codec-provided formatting failure.
        error: CodecError,
    },
    /// The adjustment rejected the initial value's normalized inverse.
    AdjustmentValueToNormalized {
        /// Adjustment-provided inverse-mapping failure.
        error: AdjustmentError,
    },
}

/// One bounded, ordered lifecycle fragment for a generic numeric text edit.
///
/// Accepted fragments are a singleton `Update`, `Commit`, or `Cancel`, or
/// `Begin` followed by one of those phases in the same transaction. Storage is
/// inline and private; the public event slice exposes only the populated
/// prefix. The text-first widget currently emits only `Begin` plus a terminal
/// event; this carrier does not implement semantic keyboard adjustment.
#[derive(Clone, Debug, PartialEq)]
pub struct NumericInputEditBatch<T> {
    events: [EditEvent<T>; 2],
    len: u8,
}

impl<T: Clone> NumericInputEditBatch<T> {
    /// The maximum number of ordered events carried by one batch.
    pub const MAX_EVENTS: usize = 2;

    /// Build a batch from one legal incremental lifecycle fragment.
    ///
    /// A singleton must be `Update`, `Commit`, or `Cancel`. A two-event
    /// fragment must begin with `Begin`, continue with one of those phases,
    /// and share one transaction; any other shape returns `None`.
    pub fn from_events(events: &[EditEvent<T>]) -> Option<Self> {
        match events {
            [event]
                if matches!(
                    event.phase,
                    EditPhase::Update | EditPhase::Commit | EditPhase::Cancel
                ) =>
            {
                Some(Self {
                    events: [event.clone(), event.clone()],
                    len: 1,
                })
            }
            [begin, next]
                if begin.phase == EditPhase::Begin
                    && matches!(
                        next.phase,
                        EditPhase::Update | EditPhase::Commit | EditPhase::Cancel
                    )
                    && begin.transaction == next.transaction =>
            {
                Some(Self {
                    events: [begin.clone(), next.clone()],
                    len: Self::MAX_EVENTS as u8,
                })
            }
            _ => None,
        }
    }

    /// Build the text-first `Begin` plus terminal batch.
    ///
    /// This helper intentionally does not accept an intermediate `Update`;
    /// use [`Self::from_events`] for an incremental fragment.
    pub fn terminal(begin: EditEvent<T>, terminal: EditEvent<T>) -> Option<Self> {
        if begin.phase != EditPhase::Begin || !terminal.phase.is_terminal() {
            return None;
        }
        Self::from_events(&[begin, terminal])
    }

    /// Return the populated ordered events in this batch.
    pub fn events(&self) -> &[EditEvent<T>] {
        &self.events[..usize::from(self.len)]
    }

    /// Return the number of populated events.
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    /// Return whether the batch contains no events.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Return the transaction carried by the populated events.
    pub const fn transaction(&self) -> EditTransaction {
        self.events[0].transaction
    }
}
