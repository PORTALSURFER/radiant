//! Caller-supplied authority revisions for controlled single-line text input.

/// Monotonic host-authority evidence for a controlled single-line text input.
///
/// The value is intentionally opaque. Applications own revision allocation and
/// must provide a strictly increasing value when a projected text value is a
/// newer authority than the retained UI-local editing state. The type is
/// qualified under `radiant::widgets`; it is not part of the common prelude.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextInputRevision(u64);

impl TextInputRevision {
    /// Build caller-supplied authority evidence.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the caller-supplied revision number.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}
