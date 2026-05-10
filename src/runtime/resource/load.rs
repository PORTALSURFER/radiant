use super::ResourceKey;

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
