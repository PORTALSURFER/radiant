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

/// One bounded, ordered terminal lifecycle for a generic numeric text edit.
///
/// The first slice carries exactly `Begin` and one terminal `Commit` or
/// `Cancel`. Storage is inline and private; the public event slice exposes only
/// the populated prefix.
#[derive(Clone, Debug, PartialEq)]
pub struct NumericInputEditBatch<T> {
    events: [EditEvent<T>; 2],
    len: u8,
}

impl<T: Clone> NumericInputEditBatch<T> {
    /// The maximum number of ordered events carried by one batch.
    pub const MAX_EVENTS: usize = 2;

    /// Build a batch from exactly `Begin` followed by `Commit` or `Cancel`.
    ///
    /// The two events must share one transaction; any other shape returns
    /// `None`.
    pub fn from_events(events: &[EditEvent<T>]) -> Option<Self> {
        let [begin, terminal] = events else {
            return None;
        };
        if begin.phase != EditPhase::Begin
            || !terminal.phase.is_terminal()
            || begin.transaction != terminal.transaction
        {
            return None;
        }

        let stored = [begin.clone(), terminal.clone()];
        Some(Self {
            events: stored,
            len: Self::MAX_EVENTS as u8,
        })
    }

    /// Build the required `Begin` plus terminal batch.
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

    /// Return the transaction shared by the populated events.
    pub const fn transaction(&self) -> EditTransaction {
        self.events[0].transaction
    }
}
