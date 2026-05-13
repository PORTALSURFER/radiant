use super::super::*;

impl GpuSurfaceRenderer {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn ensure_pipeline(
        &mut self,
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
    ) {
        let rebuild = self
            .pipeline
            .as_ref()
            .is_none_or(|pipeline| pipeline.format != target_format);
        if rebuild {
            self.pipeline = Some(GpuSurfacePipeline::new(device, target_format));
        }
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn ensure_signal_pipeline(
        &mut self,
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
    ) {
        let rebuild = self
            .signal_pipeline
            .as_ref()
            .is_none_or(|pipeline| pipeline.format != target_format);
        if rebuild {
            self.signal_pipeline = Some(SignalPipeline::new(device, target_format));
            self.signal_pipeline_generation = self.signal_pipeline_generation.wrapping_add(1);
        }
    }
}
