use std::collections::HashSet;

/// Reusable active-key set for one GPU-surface render pass.
#[derive(Default)]
pub(super) struct ActiveGpuSurfaceKeys {
    keys: HashSet<u64>,
}

impl ActiveGpuSurfaceKeys {
    pub(super) fn begin_frame(&mut self) {
        self.keys.clear();
    }

    pub(super) fn mark_active(&mut self, key: u64) {
        self.keys.insert(key);
    }

    pub(super) fn contains(&self, key: &u64) -> bool {
        self.keys.contains(key)
    }

    pub(super) fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    #[cfg(test)]
    pub(super) fn capacity(&self) -> usize {
        self.keys.capacity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_keys_reuse_storage_between_frames() {
        let mut active = ActiveGpuSurfaceKeys::default();
        active.mark_active(7);
        let initial_capacity = active.capacity();

        active.begin_frame();
        active.mark_active(8);

        assert!(initial_capacity > 0);
        assert_eq!(active.capacity(), initial_capacity);
        assert!(!active.contains(&7));
        assert!(active.contains(&8));
    }
}
