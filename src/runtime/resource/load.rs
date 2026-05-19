use super::ResourceKey;
use super::ResourceRequest;

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

/// Completion payload tagged with the request that started the resource load.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceCompletion<T> {
    /// Request token assigned before the background load started.
    pub request: ResourceRequest,
    /// Load result produced by the background work.
    pub load: ResourceLoad<T>,
}

/// Named construction fields for a [`ResourceCompletion`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceCompletionParts<T> {
    /// Request token assigned before the background load started.
    pub request: ResourceRequest,
    /// Load result produced by the background work.
    pub load: ResourceLoad<T>,
}

impl<T> ResourceCompletion<T> {
    /// Build a completion from named parts.
    pub fn from_parts(parts: ResourceCompletionParts<T>) -> Self {
        Self {
            request: parts.request,
            load: parts.load,
        }
    }

    /// Build a completion from an explicit request and load result.
    pub fn new(request: ResourceRequest, load: ResourceLoad<T>) -> Self {
        Self::from_parts(ResourceCompletionParts { request, load })
    }

    /// Return the resource key associated with this completion.
    pub fn key(&self) -> &ResourceKey {
        self.request.key()
    }

    /// Return the monotonic request generation.
    pub fn generation(&self) -> u64 {
        self.request.generation()
    }

    /// Split this completion into its request token and load result.
    pub fn into_parts(self) -> (ResourceRequest, ResourceLoad<T>) {
        (self.request, self.load)
    }

    /// Map a successful value while preserving the request token and failures.
    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> ResourceCompletion<U> {
        ResourceCompletion::from_parts(ResourceCompletionParts {
            request: self.request,
            load: self.load.map(map),
        })
    }
}
