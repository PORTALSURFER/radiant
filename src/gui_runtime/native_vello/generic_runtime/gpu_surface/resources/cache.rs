use super::super::active_keys::ActiveGpuSurfaceKeys;
use super::super::gpu_surface_types::{
    CachedSignalSummary, CachedSignalSummaryValidation, CustomShaderBinding, CustomShaderPipeline,
    GpuSurfaceCompositeBinding, GpuSurfaceTexture, SignalBodyTexture, SignalBuffer,
};
use std::collections::HashMap;

#[derive(Default)]
pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct GpuSurfaceResourceCache
{
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) textures:
        HashMap<u64, GpuSurfaceTexture>,
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
}
