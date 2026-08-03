//! Keyboard and pointer input primitives used by hotkeys and GUI backends.

use std::time::Instant;

mod key;
mod pointer;

pub use key::{KeyCode, KeyPress};
pub use pointer::logical_point_to_u16_coords;

/// Opaque monotonic timestamp captured at the native input boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct InputTimestamp(Instant);

impl InputTimestamp {
    #[allow(dead_code)]
    pub(crate) fn capture() -> Self {
        Self(Instant::now())
    }
}

#[cfg(test)]
mod tests {
    use super::InputTimestamp;

    #[test]
    fn captured_timestamps_are_nondecreasing() {
        let first = InputTimestamp::capture();
        let second = InputTimestamp::capture();

        assert!(second >= first);
    }
}
