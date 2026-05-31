#[cfg(test)]
#[path = "disclosure/tests.rs"]
mod tests;

/// Tracks one open item from a mutually exclusive group of transient UI surfaces.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExclusiveOpen<T> {
    current: Option<T>,
}

impl<T> ExclusiveOpen<T> {
    /// Create a closed exclusive-open state.
    pub const fn new() -> Self {
        Self { current: None }
    }

    /// Create an exclusive-open state from an optional item key.
    pub const fn from_open(current: Option<T>) -> Self {
        Self { current }
    }

    /// Return the currently open item key.
    pub const fn current(&self) -> Option<&T> {
        self.current.as_ref()
    }

    /// Return whether any item is open.
    pub const fn any_open(&self) -> bool {
        self.current.is_some()
    }

    /// Open one item and close any previous item.
    pub fn open(&mut self, item: T) {
        self.current = Some(item);
    }

    /// Close the current item.
    pub fn close(&mut self) {
        self.current = None;
    }

    /// Consume the state and return the current item key.
    pub fn into_option(self) -> Option<T> {
        self.current
    }
}

impl<T> ExclusiveOpen<T>
where
    T: PartialEq,
{
    /// Return whether the given item is currently open.
    pub fn is_open(&self, item: &T) -> bool {
        self.current.as_ref().is_some_and(|current| current == item)
    }

    /// Toggle one item, closing it when already open and otherwise opening it.
    pub fn toggle(&mut self, item: T) {
        if self.is_open(&item) {
            self.close();
        } else {
            self.open(item);
        }
    }
}
