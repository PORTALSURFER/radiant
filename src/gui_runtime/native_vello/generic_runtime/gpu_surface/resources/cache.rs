use super::super::super::{
    GpuSurfaceAtlasResidencySnapshot, GpuSurfaceCustomShaderResidencySnapshot,
    GpuSurfaceSignalResidencySnapshot, adapter::NativeAdapterGeneration,
};
use super::super::active_keys::ActiveGpuSurfaceKeys;
use super::super::gpu_surface_types::{
    CachedSignalSummary, CachedSignalSummaryValidation, CustomShaderBinding,
    CustomShaderBindingKey, CustomShaderPipeline, GpuSurfaceCompositeBinding,
    GpuSurfaceCompositeBindingKey, GpuSurfaceTexture, GpuSurfaceUniforms, SignalBodyTexture,
    SignalBuffer,
};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

struct AccountedMapEntry<T> {
    value: T,
    logical_bytes: Option<u64>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct AccountedMapResidency {
    resident_count: usize,
    logical_bytes: Option<u64>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CustomShaderBindingLogicalBytes {
    surface_uniform: Option<u64>,
    app_uniform: Option<u64>,
    storage: Option<u64>,
    presentation_uniform: Option<u64>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct LogicalBytesAccumulator {
    known: u128,
    unavailable: usize,
    overflowed: bool,
}

impl LogicalBytesAccumulator {
    fn add(&mut self, bytes: Option<u64>) {
        match bytes {
            Some(bytes) => {
                if let Some(known) = self.known.checked_add(u128::from(bytes)) {
                    self.known = known;
                } else {
                    self.overflowed = true;
                }
            }
            None => {
                self.unavailable = self.unavailable.saturating_add(1);
            }
        }
    }

    fn remove(&mut self, bytes: Option<u64>) {
        match bytes {
            Some(bytes) if !self.overflowed => {
                if let Some(known) = self.known.checked_sub(u128::from(bytes)) {
                    self.known = known;
                } else {
                    self.overflowed = true;
                }
            }
            Some(_) => {}
            None => {
                self.unavailable = self.unavailable.saturating_sub(1);
            }
        }
    }

    fn value(self) -> Option<u64> {
        if self.unavailable > 0 || self.overflowed {
            None
        } else {
            u64::try_from(self.known).ok()
        }
    }
}

#[derive(Default)]
struct CustomShaderResidencyAccounting {
    pipeline_resident_count: usize,
    binding_resident_count: usize,
    surface_uniform_logical_bytes: LogicalBytesAccumulator,
    app_uniform_logical_bytes: LogicalBytesAccumulator,
    storage_logical_bytes: LogicalBytesAccumulator,
    presentation_uniform_logical_bytes: LogicalBytesAccumulator,
}

impl CustomShaderResidencyAccounting {
    fn snapshot(&self) -> GpuSurfaceCustomShaderResidencySnapshot {
        GpuSurfaceCustomShaderResidencySnapshot {
            generation: NativeAdapterGeneration::default(),
            pipeline_resident_count: self.pipeline_resident_count,
            binding_resident_count: self.binding_resident_count,
            surface_uniform_logical_bytes: self.surface_uniform_logical_bytes.value(),
            app_uniform_logical_bytes: self.app_uniform_logical_bytes.value(),
            storage_logical_bytes: self.storage_logical_bytes.value(),
            presentation_uniform_logical_bytes: self.presentation_uniform_logical_bytes.value(),
        }
    }

    fn set_pipeline_resident_count(&mut self, resident_count: usize) {
        self.pipeline_resident_count = resident_count;
    }

    fn insert_binding(&mut self, logical_bytes: CustomShaderBindingLogicalBytes) {
        self.surface_uniform_logical_bytes
            .add(logical_bytes.surface_uniform);
        self.app_uniform_logical_bytes
            .add(logical_bytes.app_uniform);
        self.storage_logical_bytes.add(logical_bytes.storage);
        self.presentation_uniform_logical_bytes
            .add(logical_bytes.presentation_uniform);
    }

    fn remove_binding(&mut self, logical_bytes: CustomShaderBindingLogicalBytes) {
        self.surface_uniform_logical_bytes
            .remove(logical_bytes.surface_uniform);
        self.app_uniform_logical_bytes
            .remove(logical_bytes.app_uniform);
        self.storage_logical_bytes.remove(logical_bytes.storage);
        self.presentation_uniform_logical_bytes
            .remove(logical_bytes.presentation_uniform);
    }

    fn set_binding_resident_count(&mut self, resident_count: usize) {
        self.binding_resident_count = resident_count;
    }

    fn clear(&mut self) {
        *self = Self::default();
    }
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
        self.insert_with_bytes(key, value, logical_rgba_texel_bytes(width, height));
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn insert_with_bytes(
        &mut self,
        key: u64,
        value: T,
        logical_bytes: Option<u64>,
    ) {
        let previous = self.entries.insert(
            key,
            AccountedMapEntry {
                value,
                logical_bytes,
            },
        );
        if let Some(previous) = previous {
            self.remove_footprint(previous.logical_bytes);
        }
        self.add_footprint(logical_bytes);
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
            match entry.logical_bytes {
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
        let residency = self.residency();
        GpuSurfaceAtlasResidencySnapshot {
            generation: NativeAdapterGeneration::default(),
            resident_count: residency.resident_count,
            logical_rgba_texel_bytes: residency.logical_bytes,
        }
    }

    fn residency(&self) -> AccountedMapResidency {
        AccountedMapResidency {
            resident_count: self.resident_count,
            logical_bytes: if self.unavailable_logical_rgba_texel_bytes > 0
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
    custom_shader_residency: CustomShaderResidencyAccounting,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) signal_bodies:
        AccountedMap<SignalBodyTexture>,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) signals:
        AccountedMap<SignalBuffer>,
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
        self.custom_shader_residency
            .set_pipeline_resident_count(self.custom_shader_pipelines.len());
        let accounting = &mut self.custom_shader_residency;
        self.custom_shader_bindings.retain(|key, binding| {
            if active_keys.contains(key) {
                true
            } else {
                accounting.remove_binding(custom_shader_binding_logical_bytes(&binding.cache_key));
                false
            }
        });
        self.custom_shader_residency
            .set_binding_resident_count(self.custom_shader_bindings.len());
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
        self.custom_shader_residency.clear();
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

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn signal_residency_snapshot(
        &self,
    ) -> GpuSurfaceSignalResidencySnapshot {
        let signal_buffers = self.signals.residency();
        let signal_body_textures = self.signal_bodies.residency();
        GpuSurfaceSignalResidencySnapshot {
            generation: NativeAdapterGeneration::default(),
            signal_buffer_resident_count: signal_buffers.resident_count,
            signal_buffer_logical_bytes: signal_buffers.logical_bytes,
            signal_body_texture_resident_count: signal_body_textures.resident_count,
            signal_body_texture_logical_rgba_bytes: signal_body_textures.logical_bytes,
        }
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn insert_custom_shader_pipeline(
        &mut self,
        key: u64,
        pipeline: CustomShaderPipeline,
    ) {
        self.custom_shader_pipelines.insert(key, pipeline);
        self.custom_shader_residency
            .set_pipeline_resident_count(self.custom_shader_pipelines.len());
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn remove_custom_shader_pipeline(
        &mut self,
        key: &u64,
    ) -> Option<CustomShaderPipeline> {
        let removed = self.custom_shader_pipelines.remove(key);
        self.custom_shader_residency
            .set_pipeline_resident_count(self.custom_shader_pipelines.len());
        removed
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn insert_custom_shader_binding(
        &mut self,
        key: u64,
        binding: CustomShaderBinding,
    ) {
        let logical_bytes = custom_shader_binding_logical_bytes(&binding.cache_key);
        let previous = self.custom_shader_bindings.insert(key, binding);
        if let Some(previous) = previous {
            self.custom_shader_residency
                .remove_binding(custom_shader_binding_logical_bytes(&previous.cache_key));
        }
        self.custom_shader_residency.insert_binding(logical_bytes);
        self.custom_shader_residency
            .set_binding_resident_count(self.custom_shader_bindings.len());
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn remove_custom_shader_binding(
        &mut self,
        key: &u64,
    ) -> Option<CustomShaderBinding> {
        let removed = self.custom_shader_bindings.remove(key);
        if let Some(binding) = removed.as_ref() {
            self.custom_shader_residency
                .remove_binding(custom_shader_binding_logical_bytes(&binding.cache_key));
        }
        self.custom_shader_residency
            .set_binding_resident_count(self.custom_shader_bindings.len());
        removed
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn custom_shader_residency_snapshot(
        &self,
    ) -> GpuSurfaceCustomShaderResidencySnapshot {
        self.custom_shader_residency.snapshot()
    }
}

fn custom_shader_binding_logical_bytes(
    cache_key: &CustomShaderBindingKey,
) -> CustomShaderBindingLogicalBytes {
    CustomShaderBindingLogicalBytes {
        surface_uniform: u64::try_from(std::mem::size_of::<GpuSurfaceUniforms>()).ok(),
        app_uniform: u64::try_from(cache_key.uniform_bytes_len).ok(),
        storage: u64::try_from(cache_key.storage_bytes_len).ok(),
        presentation_uniform: u64::try_from(cache_key.presentation_uniform_bytes_len).ok(),
    }
}

pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn logical_signal_body_texture_bytes(
    width: u32,
    height: u32,
) -> Option<u64> {
    u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|texels| texels.checked_mul(4))
}

pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn logical_signal_buffer_bytes(
    sample_buffer_bytes: usize,
    uniform_buffer_bytes: usize,
) -> Option<u64> {
    u64::try_from(sample_buffer_bytes)
        .ok()?
        .checked_add(u64::try_from(uniform_buffer_bytes).ok()?)
}

#[cfg(test)]
mod tests {
    use super::{
        AccountedMap, CustomShaderBindingKey, CustomShaderBindingLogicalBytes,
        CustomShaderResidencyAccounting, NativeAdapterGeneration,
        custom_shader_binding_logical_bytes, logical_rgba_texel_bytes,
        logical_signal_body_texture_bytes, logical_signal_buffer_bytes,
    };
    use crate::gui_runtime::native_vello::generic_runtime::gpu_surface::gpu_surface_types::CustomShaderPipelineKey;
    use std::sync::Arc;

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

    #[test]
    fn custom_shader_binding_logical_bytes_track_base_and_all_optional_buffers() {
        let base = custom_shader_binding_logical_bytes(&custom_shader_binding_key(0, 0, 0));
        assert_eq!(
            base.surface_uniform,
            u64::try_from(std::mem::size_of::<super::GpuSurfaceUniforms>()).ok()
        );
        assert_eq!(base.app_uniform, Some(0));
        assert_eq!(base.storage, Some(0));
        assert_eq!(base.presentation_uniform, Some(0));

        let all_optional = custom_shader_binding_logical_bytes(&custom_shader_binding_key(4, 6, 8));
        assert_eq!(all_optional.app_uniform, Some(4));
        assert_eq!(all_optional.storage, Some(6));
        assert_eq!(all_optional.presentation_uniform, Some(8));
    }

    #[test]
    fn custom_shader_residency_accounting_keeps_counts_exact_through_replacement_and_removal() {
        let mut accounting = CustomShaderResidencyAccounting::default();
        accounting.set_pipeline_resident_count(2);

        let first = CustomShaderBindingLogicalBytes {
            surface_uniform: Some(64),
            app_uniform: Some(4),
            storage: Some(8),
            presentation_uniform: Some(12),
        };
        let replacement = CustomShaderBindingLogicalBytes {
            surface_uniform: Some(64),
            app_uniform: Some(10),
            storage: Some(0),
            presentation_uniform: Some(12),
        };
        accounting.insert_binding(first);
        accounting.set_binding_resident_count(1);
        let before_replacement = accounting.snapshot();
        accounting.remove_binding(first);
        accounting.insert_binding(replacement);
        assert_eq!(accounting.snapshot().pipeline_resident_count, 2);
        assert_eq!(accounting.snapshot().binding_resident_count, 1);
        assert_ne!(accounting.snapshot(), before_replacement);
        assert_eq!(accounting.snapshot().app_uniform_logical_bytes, Some(10));
        assert_eq!(accounting.snapshot().storage_logical_bytes, Some(0));

        accounting.remove_binding(replacement);
        accounting.set_binding_resident_count(0);
        let empty = accounting.snapshot();
        assert_eq!(empty.pipeline_resident_count, 2);
        assert_eq!(empty.binding_resident_count, 0);
        assert_eq!(empty.surface_uniform_logical_bytes, Some(0));
        assert_eq!(empty.app_uniform_logical_bytes, Some(0));
        assert_eq!(empty.storage_logical_bytes, Some(0));
        assert_eq!(empty.presentation_uniform_logical_bytes, Some(0));
    }

    #[test]
    fn custom_shader_residency_accounting_aggregates_multiple_keys_and_warm_reuse() {
        let mut accounting = CustomShaderResidencyAccounting::default();
        let first_key = CustomShaderBindingLogicalBytes {
            surface_uniform: Some(64),
            app_uniform: Some(4),
            storage: Some(8),
            presentation_uniform: Some(12),
        };
        let second_key = CustomShaderBindingLogicalBytes {
            surface_uniform: Some(64),
            app_uniform: Some(16),
            storage: Some(24),
            presentation_uniform: Some(32),
        };

        accounting.insert_binding(first_key);
        accounting.insert_binding(second_key);
        accounting.set_binding_resident_count(2);
        let aggregate = accounting.snapshot();
        assert_eq!(aggregate.binding_resident_count, 2);
        assert_eq!(aggregate.surface_uniform_logical_bytes, Some(128));
        assert_eq!(aggregate.app_uniform_logical_bytes, Some(20));
        assert_eq!(aggregate.storage_logical_bytes, Some(32));
        assert_eq!(aggregate.presentation_uniform_logical_bytes, Some(44));

        accounting.remove_binding(second_key);
        accounting.set_binding_resident_count(1);
        assert_eq!(accounting.snapshot().app_uniform_logical_bytes, Some(4));

        accounting.insert_binding(second_key);
        accounting.set_binding_resident_count(2);
        assert_eq!(accounting.snapshot(), aggregate);
    }

    #[test]
    fn custom_shader_residency_accounting_keeps_unknown_bytes_local_and_recovers() {
        let mut accounting = CustomShaderResidencyAccounting::default();
        let unknown_app = CustomShaderBindingLogicalBytes {
            surface_uniform: Some(64),
            app_uniform: None,
            storage: Some(8),
            presentation_uniform: Some(0),
        };
        accounting.insert_binding(unknown_app);
        accounting.set_binding_resident_count(1);
        let snapshot = accounting.snapshot();
        assert_eq!(snapshot.binding_resident_count, 1);
        assert_eq!(snapshot.surface_uniform_logical_bytes, Some(64));
        assert_eq!(snapshot.app_uniform_logical_bytes, None);
        assert_eq!(snapshot.storage_logical_bytes, Some(8));
        assert_eq!(snapshot.presentation_uniform_logical_bytes, Some(0));

        accounting.remove_binding(unknown_app);
        accounting.set_binding_resident_count(0);
        assert_eq!(accounting.snapshot().app_uniform_logical_bytes, Some(0));

        let aggregate_overflow = CustomShaderBindingLogicalBytes {
            surface_uniform: Some(0),
            app_uniform: Some(u64::MAX),
            storage: Some(0),
            presentation_uniform: Some(0),
        };
        let one_more = CustomShaderBindingLogicalBytes {
            app_uniform: Some(1),
            ..aggregate_overflow
        };
        accounting.insert_binding(aggregate_overflow);
        accounting.insert_binding(one_more);
        accounting.set_binding_resident_count(2);
        assert_eq!(accounting.snapshot().binding_resident_count, 2);
        assert_eq!(accounting.snapshot().app_uniform_logical_bytes, None);
        accounting.remove_binding(one_more);
        accounting.set_binding_resident_count(1);
        assert_eq!(
            accounting.snapshot().app_uniform_logical_bytes,
            Some(u64::MAX)
        );
        accounting.remove_binding(aggregate_overflow);
        accounting.set_binding_resident_count(0);
        assert_eq!(accounting.snapshot().app_uniform_logical_bytes, Some(0));
    }

    #[test]
    fn custom_shader_residency_accounting_prune_and_clear_restore_present_empty() {
        let mut accounting = CustomShaderResidencyAccounting::default();
        let footprint = CustomShaderBindingLogicalBytes {
            surface_uniform: Some(64),
            app_uniform: Some(4),
            storage: Some(8),
            presentation_uniform: Some(12),
        };
        accounting.insert_binding(footprint);
        accounting.set_binding_resident_count(1);
        accounting.remove_binding(footprint);
        accounting.set_binding_resident_count(0);
        assert_eq!(accounting.snapshot().binding_resident_count, 0);
        accounting.clear();
        assert_eq!(
            accounting.snapshot(),
            super::GpuSurfaceCustomShaderResidencySnapshot {
                generation: NativeAdapterGeneration::default(),
                pipeline_resident_count: 0,
                binding_resident_count: 0,
                surface_uniform_logical_bytes: Some(0),
                app_uniform_logical_bytes: Some(0),
                storage_logical_bytes: Some(0),
                presentation_uniform_logical_bytes: Some(0),
            }
        );
    }

    #[test]
    fn signal_resource_accounting_tracks_cold_reuse_replacement_multiple_keys_and_zero() {
        let mut buffers = AccountedMap::default();
        let mut bodies = AccountedMap::default();

        assert_eq!(buffers.residency().resident_count, 0);
        assert_eq!(buffers.residency().logical_bytes, Some(0));

        buffers.insert_with_bytes(1, "cold", Some(152));
        let warm_reuse_residency = buffers.residency();
        assert_eq!(warm_reuse_residency.resident_count, 1);
        assert_eq!(warm_reuse_residency.logical_bytes, Some(152));

        buffers.insert_with_bytes(1, "replacement", Some(304));
        buffers.insert_with_bytes(2, "zero", Some(0));
        assert_eq!(buffers.residency().resident_count, 2);
        assert_eq!(buffers.residency().logical_bytes, Some(304));

        bodies.insert_with_bytes(1, "body", Some(64 * 32 * 4));
        assert_eq!(bodies.residency().resident_count, 1);
        assert_eq!(bodies.residency().logical_bytes, Some(8_192));
    }

    fn custom_shader_binding_key(
        uniform_bytes_len: usize,
        storage_bytes_len: usize,
        presentation_uniform_bytes_len: usize,
    ) -> CustomShaderBindingKey {
        CustomShaderBindingKey {
            pipeline_key: CustomShaderPipelineKey {
                shader_key: String::from("test/custom-shader"),
                wgsl_source: Arc::<str>::from("test"),
                vertex_entry_point: String::from("vertex_main"),
                fragment_entry_point: String::from("fragment_main"),
                has_uniform_payload: uniform_bytes_len > 0,
                has_storage_payload: storage_bytes_len > 0,
                has_presentation_uniform_payload: presentation_uniform_bytes_len > 0,
            },
            uniform_bytes_len,
            storage_bytes_len,
            presentation_uniform_bytes_len,
        }
    }

    #[test]
    fn signal_resource_accounting_preserves_unknown_bytes_through_prune_and_clear() {
        let mut buffers = AccountedMap::default();
        buffers.insert_with_bytes(1, "unknown", None);
        buffers.insert_with_bytes(2, "known", Some(152));
        assert_eq!(buffers.residency().resident_count, 2);
        assert_eq!(buffers.residency().logical_bytes, None);

        buffers.retain(|key, _| *key == 2);
        assert_eq!(buffers.residency().resident_count, 1);
        assert_eq!(buffers.residency().logical_bytes, Some(152));

        buffers.clear();
        assert_eq!(buffers.residency().resident_count, 0);
        assert_eq!(buffers.residency().logical_bytes, Some(0));

        let mut bodies = AccountedMap::default();
        bodies.insert_with_bytes(1, "unknown", None);
        assert_eq!(bodies.residency().logical_bytes, None);
        bodies.clear();
        assert_eq!(bodies.residency().logical_bytes, Some(0));
    }

    #[test]
    fn signal_resource_logical_bytes_use_checked_arithmetic() {
        assert_eq!(logical_signal_body_texture_bytes(64, 32), Some(8_192));
        assert_eq!(logical_signal_body_texture_bytes(u32::MAX, u32::MAX), None);
        assert_eq!(logical_signal_buffer_bytes(8, 144), Some(152));
        assert_eq!(logical_signal_buffer_bytes(0, 0), Some(0));
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn signal_resource_logical_buffer_overflow_is_unavailable() {
        assert_eq!(logical_signal_buffer_bytes(usize::MAX, 1), None);

        let mut buffers = AccountedMap::default();
        buffers.insert_with_bytes(1, "large", Some(u64::MAX - 3));
        buffers.insert_with_bytes(2, "one-more", Some(4));
        assert_eq!(buffers.residency().logical_bytes, None);

        buffers.retain(|key, _| *key == 1);
        assert_eq!(buffers.residency().logical_bytes, Some(u64::MAX - 3));
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
