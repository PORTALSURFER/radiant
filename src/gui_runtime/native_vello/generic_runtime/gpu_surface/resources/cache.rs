use super::super::super::{GpuSurfaceAtlasResidencySnapshot, adapter::NativeAdapterGeneration};
use super::super::active_keys::ActiveGpuSurfaceKeys;
use super::super::gpu_surface_types::{
    CachedSignalSummary, CachedSignalSummaryValidation, CustomShaderBinding, CustomShaderPipeline,
    GpuSurfaceCompositeBinding, GpuSurfaceCompositeBindingKey, GpuSurfaceTexture,
    SignalBodyTexture, SignalBuffer,
};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

struct AccountedMapEntry<T> {
    value: T,
    logical_rgba_texel_bytes: Option<u64>,
}

pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct AccountedMap<T> {
    entries: HashMap<u64, AccountedMapEntry<T>>,
    resident_count: usize,
    known_logical_rgba_texel_bytes: u128,
    unavailable_logical_rgba_texel_bytes: usize,
    logical_rgba_texel_bytes_overflowed: bool,
}

#[derive(Default)]
pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct GpuSurfaceResourceFingerprintScratch
{
    atlas_entries: Vec<(
        u64,
        usize,
        u64,
        super::super::identity::RenderCanvasContentIdentity,
        usize,
        usize,
    )>,
    binding_entries: Vec<(u64, GpuSurfaceCompositeBindingKey)>,
}

impl GpuSurfaceResourceFingerprintScratch {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn reset(&mut self) {
        self.atlas_entries.clear();
        self.binding_entries.clear();
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn capacity(
        &self,
    ) -> (usize, usize) {
        (
            self.atlas_entries.capacity(),
            self.binding_entries.capacity(),
        )
    }
}

impl<T> Default for AccountedMap<T> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            resident_count: 0,
            known_logical_rgba_texel_bytes: 0,
            unavailable_logical_rgba_texel_bytes: 0,
            logical_rgba_texel_bytes_overflowed: false,
        }
    }
}

impl<T> AccountedMap<T> {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn get(
        &self,
        key: &u64,
    ) -> Option<&T> {
        self.entries.get(key).map(|entry| &entry.value)
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn insert(
        &mut self,
        key: u64,
        value: T,
        width: usize,
        height: usize,
    ) {
        let logical_rgba_texel_bytes = logical_rgba_texel_bytes(width, height);
        let previous = self.entries.insert(
            key,
            AccountedMapEntry {
                value,
                logical_rgba_texel_bytes,
            },
        );
        if let Some(previous) = previous {
            self.remove_footprint(previous.logical_rgba_texel_bytes);
        }
        self.add_footprint(logical_rgba_texel_bytes);
        self.resident_count = self.entries.len();
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn retain(
        &mut self,
        mut keep: impl FnMut(&u64, &T) -> bool,
    ) {
        let mut removed_known_logical_rgba_texel_bytes = 0_u128;
        let mut removed_unavailable_logical_rgba_texel_bytes = 0_usize;
        self.entries.retain(|key, entry| {
            if keep(key, &entry.value) {
                return true;
            }
            match entry.logical_rgba_texel_bytes {
                Some(bytes) => {
                    removed_known_logical_rgba_texel_bytes =
                        removed_known_logical_rgba_texel_bytes.saturating_add(u128::from(bytes));
                }
                None => {
                    removed_unavailable_logical_rgba_texel_bytes =
                        removed_unavailable_logical_rgba_texel_bytes.saturating_add(1);
                }
            }
            false
        });
        if !self.logical_rgba_texel_bytes_overflowed {
            if let Some(known_logical_rgba_texel_bytes) = self
                .known_logical_rgba_texel_bytes
                .checked_sub(removed_known_logical_rgba_texel_bytes)
            {
                self.known_logical_rgba_texel_bytes = known_logical_rgba_texel_bytes;
            } else {
                self.logical_rgba_texel_bytes_overflowed = true;
            }
        }
        self.unavailable_logical_rgba_texel_bytes = self
            .unavailable_logical_rgba_texel_bytes
            .saturating_sub(removed_unavailable_logical_rgba_texel_bytes);
        self.resident_count = self.entries.len();
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn clear(&mut self) {
        self.entries.clear();
        self.resident_count = 0;
        self.known_logical_rgba_texel_bytes = 0;
        self.unavailable_logical_rgba_texel_bytes = 0;
        self.logical_rgba_texel_bytes_overflowed = false;
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn is_empty(
        &self,
    ) -> bool {
        self.entries.is_empty()
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn residency_snapshot(
        &self,
    ) -> GpuSurfaceAtlasResidencySnapshot {
        GpuSurfaceAtlasResidencySnapshot {
            generation: NativeAdapterGeneration::default(),
            resident_count: self.resident_count,
            logical_rgba_texel_bytes: if self.unavailable_logical_rgba_texel_bytes > 0
                || self.logical_rgba_texel_bytes_overflowed
            {
                None
            } else {
                u64::try_from(self.known_logical_rgba_texel_bytes).ok()
            },
        }
    }

    fn add_footprint(&mut self, logical_rgba_texel_bytes: Option<u64>) {
        match logical_rgba_texel_bytes {
            Some(bytes) => {
                if let Some(known_logical_rgba_texel_bytes) = self
                    .known_logical_rgba_texel_bytes
                    .checked_add(u128::from(bytes))
                {
                    self.known_logical_rgba_texel_bytes = known_logical_rgba_texel_bytes;
                } else {
                    self.logical_rgba_texel_bytes_overflowed = true;
                }
            }
            None => {
                self.unavailable_logical_rgba_texel_bytes =
                    self.unavailable_logical_rgba_texel_bytes.saturating_add(1);
            }
        }
    }

    fn remove_footprint(&mut self, logical_rgba_texel_bytes: Option<u64>) {
        match logical_rgba_texel_bytes {
            Some(bytes) if !self.logical_rgba_texel_bytes_overflowed => {
                if let Some(known_logical_rgba_texel_bytes) = self
                    .known_logical_rgba_texel_bytes
                    .checked_sub(u128::from(bytes))
                {
                    self.known_logical_rgba_texel_bytes = known_logical_rgba_texel_bytes;
                } else {
                    self.logical_rgba_texel_bytes_overflowed = true;
                }
            }
            Some(_) => {}
            None => {
                self.unavailable_logical_rgba_texel_bytes =
                    self.unavailable_logical_rgba_texel_bytes.saturating_sub(1);
            }
        }
    }
}

impl AccountedMap<GpuSurfaceTexture> {
    fn hash_atlas_state(
        &self,
        hasher: &mut impl Hasher,
        scratch: &mut GpuSurfaceResourceFingerprintScratch,
    ) {
        scratch
            .atlas_entries
            .extend(self.entries.iter().map(|(key, entry)| {
                (
                    *key,
                    entry.value.device,
                    entry.value.revision,
                    entry.value.content_identity,
                    entry.value.width,
                    entry.value.height,
                )
            }));
        scratch.atlas_entries.sort_unstable_by_key(|entry| entry.0);
        scratch.atlas_entries.hash(hasher);
        scratch.atlas_entries.clear();
    }
}

fn logical_rgba_texel_bytes(width: usize, height: usize) -> Option<u64> {
    u64::try_from(width)
        .ok()?
        .checked_mul(u64::try_from(height).ok()?)?
        .checked_mul(4)
}

#[derive(Default)]
pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct GpuSurfaceResourceCache
{
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) textures:
        AccountedMap<GpuSurfaceTexture>,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) composite_bindings:
        HashMap<u64, GpuSurfaceCompositeBinding>,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) custom_shader_pipelines:
        HashMap<u64, CustomShaderPipeline>,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) custom_shader_bindings:
        HashMap<u64, CustomShaderBinding>,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) signal_bodies:
        HashMap<u64, SignalBodyTexture>,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) signals:
        HashMap<u64, SignalBuffer>,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) signal_summaries:
        HashMap<u64, CachedSignalSummary>,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) signal_summary_validations:
        HashMap<u64, CachedSignalSummaryValidation>,
}

impl GpuSurfaceResourceCache {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn hash_atlas_state(
        &self,
        hasher: &mut impl Hasher,
        scratch: &mut GpuSurfaceResourceFingerprintScratch,
    ) {
        self.textures.hash_atlas_state(hasher, scratch);
        scratch.binding_entries.extend(
            self.composite_bindings
                .iter()
                .map(|(key, binding)| (*key, binding.cache_key)),
        );
        scratch
            .binding_entries
            .sort_unstable_by_key(|entry| entry.0);
        scratch.binding_entries.hash(hasher);
        scratch.binding_entries.clear();
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn prune_inactive(
        &mut self,
        active_keys: &ActiveGpuSurfaceKeys,
    ) {
        self.textures.retain(|key, _| active_keys.contains(key));
        self.composite_bindings
            .retain(|key, _| active_keys.contains(key));
        self.custom_shader_pipelines
            .retain(|key, _| active_keys.contains(key));
        self.custom_shader_bindings
            .retain(|key, _| active_keys.contains(key));
        self.signal_bodies
            .retain(|key, _| active_keys.contains(key));
        self.signals.retain(|key, _| active_keys.contains(key));
        self.signal_summaries
            .retain(|key, _| active_keys.contains(key));
        self.signal_summary_validations
            .retain(|key, _| active_keys.contains(key));
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn clear(&mut self) {
        self.textures.clear();
        self.composite_bindings.clear();
        self.custom_shader_pipelines.clear();
        self.custom_shader_bindings.clear();
        self.signal_bodies.clear();
        self.signals.clear();
        self.signal_summaries.clear();
        self.signal_summary_validations.clear();
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn atlas_residency_snapshot(
        &self,
    ) -> GpuSurfaceAtlasResidencySnapshot {
        self.textures.residency_snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::{AccountedMap, NativeAdapterGeneration, logical_rgba_texel_bytes};

    #[test]
    fn accounted_map_tracks_exact_logical_rgba_texel_bytes_and_replacement() {
        let mut textures = AccountedMap::default();

        textures.insert(1, "first", 256, 128);
        assert_eq!(
            textures.residency_snapshot(),
            super::GpuSurfaceAtlasResidencySnapshot {
                generation: NativeAdapterGeneration::default(),
                resident_count: 1,
                logical_rgba_texel_bytes: Some(256 * 128 * 4),
            }
        );

        textures.insert(2, "second", 1, 1);
        textures.insert(1, "replacement", 2, 2);
        assert_eq!(textures.get(&1), Some(&"replacement"));
        assert_eq!(textures.len(), 2);
        assert_eq!(textures.residency_snapshot().resident_count, 2);
        assert_eq!(
            textures.residency_snapshot().logical_rgba_texel_bytes,
            Some(20)
        );
    }

    #[test]
    fn accounted_map_prune_and_clear_restore_exact_empty_state() {
        let mut textures = AccountedMap::default();
        textures.insert(1, "kept", 256, 128);
        textures.insert(2, "pruned", 2, 2);

        textures.retain(|key, _| *key == 1);
        assert_eq!(textures.len(), 1);
        assert_eq!(
            textures.residency_snapshot().logical_rgba_texel_bytes,
            Some(256 * 128 * 4)
        );

        textures.clear();
        assert!(textures.is_empty());
        assert_eq!(
            textures.residency_snapshot(),
            super::GpuSurfaceAtlasResidencySnapshot {
                generation: NativeAdapterGeneration::default(),
                resident_count: 0,
                logical_rgba_texel_bytes: Some(0),
            }
        );

        textures.insert(3, "unknown", usize::MAX, usize::MAX);
        assert_eq!(textures.residency_snapshot().logical_rgba_texel_bytes, None);
        textures.clear();
        assert_eq!(
            textures.residency_snapshot().logical_rgba_texel_bytes,
            Some(0)
        );
    }

    #[test]
    fn accounted_map_keeps_unknown_footprints_unavailable_until_last_unknown_is_removed() {
        let mut textures = AccountedMap::default();
        textures.insert(1, "unknown", usize::MAX, usize::MAX);
        textures.insert(2, "known", 256, 128);
        assert_eq!(textures.residency_snapshot().resident_count, 2);
        assert_eq!(textures.residency_snapshot().logical_rgba_texel_bytes, None);

        textures.retain(|key, _| *key == 2);
        assert_eq!(
            textures.residency_snapshot().logical_rgba_texel_bytes,
            Some(256 * 128 * 4)
        );

        textures.insert(2, "unknown replacement", usize::MAX, usize::MAX);
        assert_eq!(textures.residency_snapshot().logical_rgba_texel_bytes, None);
        textures.insert(2, "known replacement", 256, 128);
        assert_eq!(
            textures.residency_snapshot().logical_rgba_texel_bytes,
            Some(256 * 128 * 4)
        );
    }

    #[test]
    fn logical_rgba_texel_bytes_uses_checked_u64_math() {
        assert_eq!(logical_rgba_texel_bytes(256, 128), Some(131_072));
        assert_eq!(logical_rgba_texel_bytes(usize::MAX, usize::MAX), None);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn accounted_map_keeps_u64_aggregate_overflow_unavailable_until_pruned() {
        let mut textures = AccountedMap::default();
        let width = (u64::MAX / 4) as usize;
        textures.insert(1, "large", width, 1);
        textures.insert(2, "one-more", 1, 1);

        assert_eq!(textures.residency_snapshot().logical_rgba_texel_bytes, None);

        textures.retain(|key, _| *key == 1);
        assert_eq!(
            textures.residency_snapshot().logical_rgba_texel_bytes,
            Some(u64::MAX - 3)
        );
    }
}
