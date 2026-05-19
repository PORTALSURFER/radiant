use super::{ResourceKey, ResourceLoad, ResourceRequest};

mod state;

pub use state::ResourceLoadState;

/// Stored state for one host-owned resource.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceSlot<T> {
    key: ResourceKey,
    state: ResourceLoadState,
    value: Option<T>,
    error: Option<String>,
    revision: u64,
    generation: u64,
}

impl<T> ResourceSlot<T> {
    /// Build an idle resource slot.
    pub fn new(key: impl Into<ResourceKey>) -> Self {
        Self {
            key: key.into(),
            state: ResourceLoadState::Idle,
            value: None,
            error: None,
            revision: 0,
            generation: 0,
        }
    }

    /// Return the stable key for this slot.
    pub fn key(&self) -> &ResourceKey {
        &self.key
    }

    /// Return the current loading state.
    pub fn state(&self) -> ResourceLoadState {
        self.state
    }

    /// Return whether a background load is in progress.
    pub fn is_loading(&self) -> bool {
        self.state == ResourceLoadState::Loading
    }

    /// Return the latest successfully loaded value, if any.
    pub fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// Return the latest load error, if any.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Return the monotonic revision for completed loads and clears.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Mark this resource as loading and return a request token.
    ///
    /// Applying a later completion with [`Self::apply_for`] rejects older
    /// tokens for the same resource key.
    pub fn begin_load(&mut self) -> ResourceRequest {
        self.generation = self.generation.saturating_add(1);
        self.state = ResourceLoadState::Loading;
        self.error = None;
        ResourceRequest::new(self.key.clone(), self.generation)
    }

    /// Mark this resource as loading and clear the previous error.
    pub fn mark_loading(&mut self) {
        let _ = self.begin_load();
    }

    /// Clear loaded value and error state.
    pub fn clear(&mut self) {
        self.generation = self.generation.saturating_add(1);
        self.state = ResourceLoadState::Idle;
        self.value = None;
        self.error = None;
        self.bump_revision();
    }

    /// Cancel the active load request while preserving any last ready value.
    ///
    /// This invalidates the current request generation so later completions for
    /// the canceled work are ignored by [`Self::apply_for`]. Unlike
    /// [`Self::clear`], this keeps the last successful value available and
    /// returns the slot to `Ready` when such a value exists.
    pub fn cancel_load(&mut self) {
        self.generation = self.generation.saturating_add(1);
        self.error = None;
        self.state = if self.value.is_some() {
            ResourceLoadState::Ready
        } else {
            ResourceLoadState::Idle
        };
        self.bump_revision();
    }

    /// Apply a completed load result.
    ///
    /// Results for another key are ignored and return `false`.
    pub fn apply(&mut self, load: ResourceLoad<T>) -> bool {
        if load.key() != &self.key {
            return false;
        }

        match load {
            ResourceLoad::Ready { value, .. } => {
                self.state = ResourceLoadState::Ready;
                self.value = Some(value);
                self.error = None;
            }
            ResourceLoad::Failed { error, .. } => {
                self.state = ResourceLoadState::Failed;
                self.value = None;
                self.error = Some(error);
            }
        }
        self.bump_revision();
        true
    }

    /// Apply a completed load only if it belongs to the current request token.
    ///
    /// Results for another key or an older request generation are ignored and
    /// return `false`.
    pub fn apply_for(&mut self, request: &ResourceRequest, load: ResourceLoad<T>) -> bool {
        if request.key() != &self.key || request.generation() != self.generation {
            return false;
        }
        self.apply(load)
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }
}

impl<T> Default for ResourceSlot<T> {
    fn default() -> Self {
        Self::new("default")
    }
}

#[cfg(test)]
#[path = "slot/tests.rs"]
mod tests;
