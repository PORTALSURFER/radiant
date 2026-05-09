//! Domain-neutral resource loading state for runtime-managed background work.

use std::fmt;

/// Stable host-defined key for one loadable resource.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceKey(String);

impl ResourceKey {
    /// Build a resource key from host-owned text.
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    /// Return the key text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&'static str> for ResourceKey {
    fn from(value: &'static str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ResourceKey {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for ResourceKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

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

/// Result message produced by host-owned resource work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceLoad<T> {
    /// The resource loaded successfully.
    Ready {
        /// Key of the loaded resource.
        key: ResourceKey,
        /// Loaded host-owned value.
        value: T,
    },
    /// The resource failed to load.
    Failed {
        /// Key of the failed resource.
        key: ResourceKey,
        /// Human-readable failure detail.
        error: String,
    },
}

impl<T> ResourceLoad<T> {
    /// Build a successful resource load result.
    pub fn ready(key: impl Into<ResourceKey>, value: T) -> Self {
        Self::Ready {
            key: key.into(),
            value,
        }
    }

    /// Build a failed resource load result.
    pub fn failed(key: impl Into<ResourceKey>, error: impl Into<String>) -> Self {
        Self::Failed {
            key: key.into(),
            error: error.into(),
        }
    }

    /// Return the resource key associated with this load result.
    pub fn key(&self) -> &ResourceKey {
        match self {
            Self::Ready { key, .. } | Self::Failed { key, .. } => key,
        }
    }

    /// Map a successful value while preserving the resource key and failures.
    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> ResourceLoad<U> {
        match self {
            Self::Ready { key, value } => ResourceLoad::Ready {
                key,
                value: map(value),
            },
            Self::Failed { key, error } => ResourceLoad::Failed { key, error },
        }
    }
}

/// Stored state for one host-owned resource.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceSlot<T> {
    key: ResourceKey,
    state: ResourceLoadState,
    value: Option<T>,
    error: Option<String>,
    revision: u64,
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

    /// Mark this resource as loading and clear the previous error.
    pub fn mark_loading(&mut self) {
        self.state = ResourceLoadState::Loading;
        self.error = None;
    }

    /// Clear loaded value and error state.
    pub fn clear(&mut self) {
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
}
