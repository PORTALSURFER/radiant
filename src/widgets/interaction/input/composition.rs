//! Backend-neutral text-composition samples and bounded scalar ranges.

use crate::gui::input::InputTimestamp;
use std::ops::Range;

/// A bounded half-open range indexed by Unicode scalar values.
///
/// The range stores the scalar length it was validated against.  Consumers
/// can therefore reject a range when the text it describes changes without
/// silently rebasing byte, UTF-16, or grapheme offsets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CompositionRange {
    start: usize,
    end: usize,
    scalar_len: usize,
}

/// Error returned when a composition scalar range is malformed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CompositionRangeError {
    /// The range is inverted rather than half-open in ascending order.
    Inverted {
        /// Inclusive first endpoint supplied by the caller.
        start: usize,
        /// Exclusive second endpoint supplied by the caller.
        end: usize,
    },
    /// An endpoint is outside the validated scalar length.
    OutOfBounds {
        /// Inclusive first endpoint supplied by the caller.
        start: usize,
        /// Exclusive second endpoint supplied by the caller.
        end: usize,
        /// Scalar length against which the endpoints were checked.
        scalar_len: usize,
    },
}

impl CompositionRange {
    /// Construct a validated half-open scalar range `[start, end)`.
    pub fn new(start: usize, end: usize, scalar_len: usize) -> Result<Self, CompositionRangeError> {
        if start > end {
            return Err(CompositionRangeError::Inverted { start, end });
        }
        if end > scalar_len {
            return Err(CompositionRangeError::OutOfBounds {
                start,
                end,
                scalar_len,
            });
        }
        Ok(Self {
            start,
            end,
            scalar_len,
        })
    }

    /// Alias for [`Self::new`] for callers that prefer fallible constructor
    /// naming consistent with other backend-neutral input values.
    pub fn try_new(
        start: usize,
        end: usize,
        scalar_len: usize,
    ) -> Result<Self, CompositionRangeError> {
        Self::new(start, end, scalar_len)
    }

    /// Construct a validated scalar range from Rust's half-open range type.
    pub fn from_range(
        range: Range<usize>,
        scalar_len: usize,
    ) -> Result<Self, CompositionRangeError> {
        Self::new(range.start, range.end, scalar_len)
    }

    /// Return the first scalar endpoint.
    pub const fn start(self) -> usize {
        self.start
    }

    /// Return the exclusive scalar endpoint.
    pub const fn end(self) -> usize {
        self.end
    }

    /// Return the scalar length used to validate this range.
    pub const fn scalar_len(self) -> usize {
        self.scalar_len
    }

    /// Return the number of scalars in this range.
    pub const fn len(self) -> usize {
        self.end - self.start
    }

    /// Return whether this range is empty or collapsed.
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Return whether this range is a collapsed caret.
    pub const fn is_collapsed(self) -> bool {
        self.start == self.end
    }

    /// Return the ordinary half-open range representation.
    pub const fn as_range(self) -> Range<usize> {
        self.start..self.end
    }

    /// Return whether the endpoints remain valid for another scalar length.
    pub fn is_valid_for(self, scalar_len: usize) -> bool {
        self.start <= self.end && self.end <= scalar_len && self.scalar_len == scalar_len
    }
}

/// Exact focused text context captured when native composition begins.
///
/// Both ranges use the committed value's Unicode-scalar coordinates. Native
/// adapters obtain this value from the focused widget rather than deriving a
/// replacement range from preedit text or native byte offsets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CompositionStartContext {
    replacement_range: CompositionRange,
    selection: CompositionRange,
}

impl CompositionStartContext {
    /// Construct a focused composition context with matching scalar lengths.
    pub fn new(
        replacement_range: CompositionRange,
        selection: CompositionRange,
    ) -> Result<Self, CompositionSampleError> {
        if replacement_range.scalar_len() != selection.scalar_len() {
            return Err(CompositionSampleError::StartRangeScalarLengthMismatch {
                replacement: replacement_range.scalar_len(),
                selection: selection.scalar_len(),
            });
        }
        Ok(Self {
            replacement_range,
            selection,
        })
    }

    /// Return the committed-value scalar range replaced on commit.
    pub const fn replacement_range(self) -> CompositionRange {
        self.replacement_range
    }

    /// Return the committed-value scalar selection captured at start.
    pub const fn selection(self) -> CompositionRange {
        self.selection
    }
}

/// Runtime-only state for native preedit selection evidence.
///
/// `Unreported` is the state immediately after `Start`, before the adapter has
/// delivered a preedit selection.  It is intentionally distinct from
/// `Hidden`, which is explicit native evidence that caret and selection paint
/// must be suppressed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CompositionSelectionState {
    /// No preedit selection has been reported yet.
    #[default]
    Unreported,
    /// The adapter reported this exact visible preedit selection.
    Visible(CompositionRange),
    /// The adapter explicitly reported that the preedit selection is hidden.
    Hidden,
}

impl CompositionSelectionState {
    #[cfg(test)]
    pub(crate) const fn visible_range(self) -> Option<CompositionRange> {
        match self {
            Self::Visible(range) => Some(range),
            Self::Unreported | Self::Hidden => None,
        }
    }

    pub(crate) const fn is_hidden(self) -> bool {
        matches!(self, Self::Hidden)
    }
}

/// Lifecycle phase carried by one validated composition sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CompositionPhase {
    /// Begin a new composition and capture the focused widget's ranges.
    Start,
    /// Replace the transient preedit and report its scalar selection.
    Update,
    /// Atomically commit text through the owning widget.
    Commit,
    /// Discard transient composition state through the owning widget.
    Cancel,
}

/// Error returned when composition range evidence does not match its text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CompositionSampleError {
    /// The replacement range supplied for `Start` is malformed.
    ReplacementRange(CompositionRangeError),
    /// The start selection supplied for `Start` is malformed.
    StartSelection(CompositionRangeError),
    /// The start ranges were validated against different scalar lengths.
    StartRangeScalarLengthMismatch {
        /// Scalar length captured by the replacement range.
        replacement: usize,
        /// Scalar length captured by the start selection.
        selection: usize,
    },
    /// The update selection is malformed for the supplied preedit text.
    UpdateSelection(CompositionRangeError),
    /// The update selection was validated against a different scalar length.
    UpdateSelectionScalarLengthMismatch {
        /// Scalar length captured by the selection.
        selection: usize,
        /// Actual Unicode-scalar length of the supplied preedit.
        preedit: usize,
    },
}

/// Validated backend-neutral composition input.
///
/// `Start` and `Update` carry only Unicode-scalar ranges. Native adapters may
/// preserve an exact [`InputTimestamp`] when one exists; synthetic constructors
/// intentionally leave it absent. No sequence identity is fabricated by this
/// vocabulary.
#[derive(Clone, Debug, PartialEq)]
pub enum CompositionSample {
    /// Begin a composition over a captured committed-text range.
    Start {
        /// Committed-text scalar range that will be replaced on `Commit`.
        replacement_range: CompositionRange,
        /// Scalar selection captured at composition start.
        selection: CompositionRange,
        /// Optional exact native timestamp.
        timestamp: Option<InputTimestamp>,
    },
    /// Replace the visible preedit without mutating committed text.
    Update {
        /// Verbatim transient preedit text.
        preedit: String,
        /// Scalar selection inside `preedit`.
        selection: CompositionRange,
        /// Optional exact native timestamp.
        timestamp: Option<InputTimestamp>,
    },
    /// Commit text through the owner.
    Commit {
        /// Verbatim text to commit.
        text: String,
        /// Optional exact native timestamp.
        timestamp: Option<InputTimestamp>,
    },
    /// Cancel the current composition through the owner.
    Cancel {
        /// Optional exact native timestamp.
        timestamp: Option<InputTimestamp>,
    },
}

impl CompositionSample {
    /// Construct a synthetic timestamp-free `Start` sample.
    pub fn start(
        replacement_range: CompositionRange,
        selection: CompositionRange,
    ) -> Result<Self, CompositionSampleError> {
        Self::start_with_metadata(replacement_range, selection, None)
    }

    /// Construct `Start` while preserving an exact native timestamp.
    pub fn start_with_timestamp(
        replacement_range: CompositionRange,
        selection: CompositionRange,
        timestamp: InputTimestamp,
    ) -> Result<Self, CompositionSampleError> {
        Self::start_with_metadata(replacement_range, selection, Some(timestamp))
    }

    /// Construct `Start` while preserving optional exact native metadata.
    pub fn start_with_metadata(
        replacement_range: CompositionRange,
        selection: CompositionRange,
        timestamp: Option<InputTimestamp>,
    ) -> Result<Self, CompositionSampleError> {
        if replacement_range.scalar_len() != selection.scalar_len() {
            return Err(CompositionSampleError::StartRangeScalarLengthMismatch {
                replacement: replacement_range.scalar_len(),
                selection: selection.scalar_len(),
            });
        }
        Ok(Self::Start {
            replacement_range,
            selection,
            timestamp,
        })
    }

    /// Construct a synthetic timestamp-free `Update` sample.
    pub fn update(
        preedit: impl Into<String>,
        selection: CompositionRange,
    ) -> Result<Self, CompositionSampleError> {
        Self::update_with_metadata(preedit, selection, None)
    }

    /// Construct `Update` while preserving an exact native timestamp.
    pub fn update_with_timestamp(
        preedit: impl Into<String>,
        selection: CompositionRange,
        timestamp: InputTimestamp,
    ) -> Result<Self, CompositionSampleError> {
        Self::update_with_metadata(preedit, selection, Some(timestamp))
    }

    /// Construct `Update` while preserving optional exact native metadata.
    pub fn update_with_metadata(
        preedit: impl Into<String>,
        selection: CompositionRange,
        timestamp: Option<InputTimestamp>,
    ) -> Result<Self, CompositionSampleError> {
        let preedit = preedit.into();
        let preedit_scalar_len = preedit.chars().count();
        if selection.scalar_len() != preedit_scalar_len {
            return Err(
                CompositionSampleError::UpdateSelectionScalarLengthMismatch {
                    selection: selection.scalar_len(),
                    preedit: preedit_scalar_len,
                },
            );
        }
        if selection.end() > preedit_scalar_len {
            return Err(CompositionSampleError::UpdateSelection(
                CompositionRangeError::OutOfBounds {
                    start: selection.start(),
                    end: selection.end(),
                    scalar_len: preedit_scalar_len,
                },
            ));
        }
        Ok(Self::Update {
            preedit,
            selection,
            timestamp,
        })
    }

    /// Construct a synthetic timestamp-free `Commit` sample.
    pub fn commit(text: impl Into<String>) -> Self {
        Self::commit_with_metadata(text, None)
    }

    /// Construct `Commit` while preserving an exact native timestamp.
    pub fn commit_with_timestamp(text: impl Into<String>, timestamp: InputTimestamp) -> Self {
        Self::commit_with_metadata(text, Some(timestamp))
    }

    /// Construct `Commit` while preserving optional exact native metadata.
    pub fn commit_with_metadata(
        text: impl Into<String>,
        timestamp: Option<InputTimestamp>,
    ) -> Self {
        Self::Commit {
            text: text.into(),
            timestamp,
        }
    }

    /// Construct a synthetic timestamp-free `Cancel` sample.
    pub const fn cancel() -> Self {
        Self::Cancel { timestamp: None }
    }

    /// Construct `Cancel` while preserving an exact native timestamp.
    pub const fn cancel_with_timestamp(timestamp: InputTimestamp) -> Self {
        Self::Cancel {
            timestamp: Some(timestamp),
        }
    }

    /// Construct `Cancel` while preserving optional exact native metadata.
    pub const fn cancel_with_metadata(timestamp: Option<InputTimestamp>) -> Self {
        Self::Cancel { timestamp }
    }

    /// Return this sample's lifecycle phase.
    pub const fn phase(&self) -> CompositionPhase {
        match self {
            Self::Start { .. } => CompositionPhase::Start,
            Self::Update { .. } => CompositionPhase::Update,
            Self::Commit { .. } => CompositionPhase::Commit,
            Self::Cancel { .. } => CompositionPhase::Cancel,
        }
    }

    /// Return whether the sample still satisfies its range/text invariants.
    ///
    /// The constructors enforce these rules, while this query also protects
    /// callers that build a public enum variant directly.
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Start {
                replacement_range,
                selection,
                ..
            } => replacement_range.scalar_len() == selection.scalar_len(),
            Self::Update {
                preedit, selection, ..
            } => {
                let scalar_len = preedit.chars().count();
                selection.is_valid_for(scalar_len)
            }
            Self::Commit { .. } | Self::Cancel { .. } => true,
        }
    }

    /// Return the exact optional timestamp carried by this sample.
    pub const fn timestamp(&self) -> Option<InputTimestamp> {
        match self {
            Self::Start { timestamp, .. }
            | Self::Update { timestamp, .. }
            | Self::Commit { timestamp, .. }
            | Self::Cancel { timestamp } => *timestamp,
        }
    }

    /// Return the captured replacement range for `Start` samples.
    pub const fn replacement_range(&self) -> Option<CompositionRange> {
        match self {
            Self::Start {
                replacement_range, ..
            } => Some(*replacement_range),
            Self::Update { .. } | Self::Commit { .. } | Self::Cancel { .. } => None,
        }
    }

    /// Return the scalar selection carried by `Start` or `Update`.
    pub const fn selection(&self) -> Option<CompositionRange> {
        match self {
            Self::Start { selection, .. } | Self::Update { selection, .. } => Some(*selection),
            Self::Commit { .. } | Self::Cancel { .. } => None,
        }
    }

    /// Return the transient preedit carried by `Update`.
    pub fn preedit(&self) -> Option<&str> {
        match self {
            Self::Update { preedit, .. } => Some(preedit),
            Self::Start { .. } | Self::Commit { .. } | Self::Cancel { .. } => None,
        }
    }

    /// Return the committed text carried by `Commit`.
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Commit { text, .. } => Some(text),
            Self::Start { .. } | Self::Update { .. } | Self::Cancel { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_ranges_are_bounded_half_open_values() {
        let range = CompositionRange::new(1, 3, 4).expect("valid scalar range");
        assert_eq!(range.as_range(), 1..3);
        assert_eq!(range.len(), 2);
        assert!(!range.is_collapsed());
        assert!(CompositionRange::new(3, 1, 4).is_err());
        assert!(CompositionRange::new(1, 5, 4).is_err());
    }

    #[test]
    fn update_validation_uses_unicode_scalar_length_and_keeps_synthetic_timestamp_absent() {
        let selection = CompositionRange::new(1, 1, 2).expect("two scalar preedit");
        let sample = CompositionSample::update("あい", selection).expect("valid update");
        assert_eq!(sample.preedit(), Some("あい"));
        assert_eq!(sample.selection(), Some(selection));
        assert_eq!(sample.timestamp(), None);
        assert!(
            CompositionSample::update(
                "あ",
                CompositionRange::new(0, 1, 2).expect("range with mismatched evidence"),
            )
            .is_err()
        );
    }

    #[test]
    fn focused_start_context_keeps_both_exact_scalar_ranges() {
        let replacement = CompositionRange::new(1, 3, 4).expect("valid replacement");
        let selection = CompositionRange::new(2, 2, 4).expect("valid selection");
        let context = CompositionStartContext::new(replacement, selection)
            .expect("matching scalar lengths should be accepted");

        assert_eq!(context.replacement_range(), replacement);
        assert_eq!(context.selection(), selection);
        assert!(
            CompositionStartContext::new(
                replacement,
                CompositionRange::new(0, 0, 3).expect("different scalar length"),
            )
            .is_err()
        );
    }

    #[test]
    fn direct_commit_and_cancel_constructors_are_timestamp_free() {
        assert_eq!(CompositionSample::commit("text").text(), Some("text"));
        assert_eq!(CompositionSample::commit("text").timestamp(), None);
        assert_eq!(
            CompositionSample::cancel().phase(),
            CompositionPhase::Cancel
        );
        assert_eq!(CompositionSample::cancel().timestamp(), None);
    }

    #[test]
    fn exact_timestamp_constructors_preserve_the_supplied_timestamp() {
        let timestamp = InputTimestamp::capture();
        let range = CompositionRange::new(0, 0, 0).expect("empty scalar range");
        let start = CompositionSample::start_with_timestamp(range, range, timestamp)
            .expect("valid timestamped start");
        assert_eq!(start.timestamp(), Some(timestamp));
        assert_eq!(
            CompositionSample::commit_with_timestamp("text", timestamp).timestamp(),
            Some(timestamp)
        );
        assert_eq!(
            CompositionSample::cancel_with_timestamp(timestamp).timestamp(),
            Some(timestamp)
        );
    }
}
