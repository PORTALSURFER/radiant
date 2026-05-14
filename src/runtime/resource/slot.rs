use super::{ResourceKey, ResourceLoad, ResourceRequest};

/// Current state of a resource slot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ResourceLoadState {
    /// No load has been requested.
    #[default]
    Idle,
    /// A background load is running.
    Loading,
    /// The latest load completed successfully.
    Ready,
    /// The latest load failed.
    Failed,
}

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
mod tests {
    use super::{ResourceLoad, ResourceLoadState, ResourceSlot};

    #[test]
    fn resource_slot_tracks_loading_success_failure_and_revision() {
        let mut slot = ResourceSlot::new("preview");

        assert_eq!(slot.state(), ResourceLoadState::Idle);
        assert_eq!(slot.revision(), 0);

        slot.mark_loading();
        assert!(slot.is_loading());
        assert_eq!(slot.error(), None);

        assert!(slot.apply(ResourceLoad::ready("preview", "pixels")));
        assert_eq!(slot.state(), ResourceLoadState::Ready);
        assert_eq!(slot.value(), Some(&"pixels"));
        assert_eq!(slot.revision(), 1);

        assert!(slot.apply(ResourceLoad::failed("preview", "decode failed")));
        assert_eq!(slot.state(), ResourceLoadState::Failed);
        assert_eq!(slot.value(), None);
        assert_eq!(slot.error(), Some("decode failed"));
        assert_eq!(slot.revision(), 2);
    }

    #[test]
    fn resource_slot_ignores_results_for_other_keys() {
        let mut slot = ResourceSlot::<String>::new("preview");

        assert!(!slot.apply(ResourceLoad::ready("other", String::from("stale"))));
        assert_eq!(slot.state(), ResourceLoadState::Idle);
        assert_eq!(slot.revision(), 0);
    }

    #[test]
    fn resource_slot_rejects_stale_request_results_for_same_key() {
        let mut slot = ResourceSlot::new("preview");

        let stale = slot.begin_load();
        let current = slot.begin_load();

        assert_eq!(stale.key().as_str(), "preview");
        assert!(current.generation() > stale.generation());
        assert!(!slot.apply_for(&stale, ResourceLoad::ready("preview", "old pixels")));
        assert_eq!(slot.state(), ResourceLoadState::Loading);
        assert_eq!(slot.revision(), 0);

        assert!(slot.apply_for(&current, ResourceLoad::ready("preview", "new pixels")));
        assert_eq!(slot.state(), ResourceLoadState::Ready);
        assert_eq!(slot.value(), Some(&"new pixels"));
        assert_eq!(slot.revision(), 1);
    }

    #[test]
    fn resource_request_builds_keyed_success_and_failure_results() {
        let mut slot = ResourceSlot::new("preview");
        let request = slot.begin_load();

        assert!(slot.apply_for(&request, request.ready("pixels")));
        assert_eq!(slot.value(), Some(&"pixels"));

        let request = slot.begin_load();
        assert!(slot.apply_for(&request, request.failed("decode failed")));
        assert_eq!(slot.state(), ResourceLoadState::Failed);
        assert_eq!(slot.error(), Some("decode failed"));
    }

    #[test]
    fn resource_slot_clear_invalidates_in_flight_request() {
        let mut slot = ResourceSlot::new("preview");

        let request = slot.begin_load();
        slot.clear();

        assert!(!slot.apply_for(&request, ResourceLoad::ready("preview", "pixels")));
        assert_eq!(slot.state(), ResourceLoadState::Idle);
        assert_eq!(slot.value(), None);
        assert_eq!(slot.revision(), 1);
    }
}
