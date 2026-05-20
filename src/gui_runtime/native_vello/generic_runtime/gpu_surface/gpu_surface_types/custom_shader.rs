use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct CustomShaderPipelineKey
{
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) shader_key: String,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) wgsl_source: Arc<str>,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) vertex_entry_point:
        String,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fragment_entry_point:
        String,
}

pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct CustomShaderPipeline {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) format:
        wgpu::TextureFormat,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) device: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) key:
        CustomShaderPipelineKey,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) bind_group_layout:
        wgpu::BindGroupLayout,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) pipeline:
        wgpu::RenderPipeline,
}

impl CustomShaderPipeline {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn matches(
        &self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        key: &CustomShaderPipelineKey,
    ) -> bool {
        wgpu_target_matches(self.device, self.format, device, format) && self.key == *key
    }
}

pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct CustomShaderBinding {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) cache_key:
        CustomShaderPipelineKey,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) uniform_buffer:
        wgpu::Buffer,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) bind_group:
        wgpu::BindGroup,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_shader_pipeline_key_tracks_shader_stage_contract() {
        let key = CustomShaderPipelineKey {
            shader_key: String::from("meter"),
            wgsl_source: Arc::<str>::from("@vertex fn vertex_main() {}"),
            vertex_entry_point: String::from("vertex_main"),
            fragment_entry_point: String::from("fragment_main"),
        };

        assert_ne!(
            key,
            CustomShaderPipelineKey {
                fragment_entry_point: String::from("other_fragment"),
                ..key.clone()
            }
        );
        assert_ne!(
            key.clone(),
            CustomShaderPipelineKey {
                wgsl_source: Arc::<str>::from("@vertex fn other_vertex() {}"),
                ..key
            }
        );
    }
}
