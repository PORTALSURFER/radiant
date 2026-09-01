//! Keyboard and pointer input primitives used by hotkeys and GUI backends.

use std::time::Instant;

mod key;
mod pointer;

pub use key::{KeyCode, KeyPress};
pub use pointer::logical_point_to_u16_coords;

/// Opaque monotonic timestamp captured at the native input boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InputTimestamp(Instant);

impl InputTimestamp {
    pub(crate) const fn from_instant(instant: Instant) -> Self {
        Self(instant)
    }

    pub(crate) const fn instant(self) -> Instant {
        self.0
    }

    #[allow(dead_code)]
    pub(crate) fn capture() -> Self {
        Self::from_instant(Instant::now())
    }
}

/// Opaque sequence identity allocated for one accepted native input sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InputSequence(u64);

impl InputSequence {
    pub(crate) const fn from_runtime_value(value: u64) -> Self {
        Self(value)
    }

    #[allow(dead_code)]
    pub(crate) const fn runtime_value(self) -> u64 {
        self.0
    }
}

/// Opaque range of native input samples contributed to one delivery.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InputSequenceRange {
    start: InputSequence,
    end: InputSequence,
}

impl InputSequenceRange {
    pub(crate) const fn singleton(sequence: InputSequence) -> Self {
        Self {
            start: sequence,
            end: sequence,
        }
    }

    #[allow(dead_code)]
    pub(crate) const fn start(self) -> InputSequence {
        self.start
    }

    pub(crate) const fn end(self) -> InputSequence {
        self.end
    }

    pub(crate) fn extend_end(&mut self, sequence: InputSequence) {
        self.end = sequence;
    }
}

#[cfg(test)]
mod tests {
    use super::{InputSequence, InputSequenceRange, InputTimestamp};

    #[test]
    fn captured_timestamps_are_nondecreasing() {
        let first = InputTimestamp::capture();
        let second = InputTimestamp::capture();

        assert!(second >= first);
    }

    #[test]
    fn sequence_ranges_are_singletons_and_extend_only_the_latest_endpoint() {
        let first = InputSequence::from_runtime_value(4);
        let newest = InputSequence::from_runtime_value(9);
        let mut range = InputSequenceRange::singleton(first);

        assert_eq!(range.start().runtime_value(), 4);
        assert_eq!(range.end().runtime_value(), 4);

        range.extend_end(newest);

        assert_eq!(range.start().runtime_value(), 4);
        assert_eq!(range.end().runtime_value(), 9);
    }
}
