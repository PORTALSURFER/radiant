use super::*;

pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct GpuSurfacePipeline {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) format:
        wgpu::TextureFormat,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) device: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) bind_group_layout:
        wgpu::BindGroupLayout,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) pipeline:
        wgpu::RenderPipeline,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) sampler: wgpu::Sampler,
}

impl GpuSurfacePipeline {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn matches_target(
        &self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> bool {
        wgpu_target_matches(self.device, self.format, device, format)
    }
}

pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct SignalPipeline {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) format:
        wgpu::TextureFormat,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) device: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) bind_group_layout:
        wgpu::BindGroupLayout,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) pipeline:
        wgpu::RenderPipeline,
}

impl SignalPipeline {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn matches_target(
        &self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> bool {
        wgpu_target_matches(self.device, self.format, device, format)
    }
}
