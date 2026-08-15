//! Monotonic declarative inputs for runtime-owned layout state.

/// A value paired with the generation that produced it.
///
/// Consumers decide how generations are reconciled. The split-pane runtime
/// contract accepts a controlled value on mount and only strictly newer
/// generations after that point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Controlled<T> {
    value: T,
    generation: u64,
}

impl<T> Controlled<T> {
    /// Construct a controlled value with its caller-owned generation.
    pub const fn new(value: T, generation: u64) -> Self {
        Self { value, generation }
    }

    /// Borrow the controlled value.
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// Return the caller-owned generation.
    pub const fn generation(&self) -> u64 {
        self.generation
    }
}
