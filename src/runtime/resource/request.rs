use super::ResourceKey;

/// Request token for one in-flight resource load.
///
/// Hosts that can start repeated loads for the same key should keep this token
/// with the worker result and apply it through [`crate::runtime::ResourceSlot::apply_for`] so an
/// older completion cannot overwrite a newer request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceRequest {
    key: ResourceKey,
    generation: u64,
}

impl ResourceRequest {
    pub(super) fn new(key: ResourceKey, generation: u64) -> Self {
        Self { key, generation }
    }

    /// Return the resource key associated with this request.
    pub fn key(&self) -> &ResourceKey {
        &self.key
    }

    /// Return the monotonic request generation.
    pub fn generation(&self) -> u64 {
        self.generation
    }
}
