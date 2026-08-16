use super::super::wgpu_target_matches;
use std::sync::Arc;
use vello::wgpu;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct CustomShaderPipelineKey
{
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) shader_key: String,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) wgsl_source: Arc<str>,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) vertex_entry_point:
        String,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fragment_entry_point:
        String,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) has_uniform_payload:
        bool,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) has_storage_payload:
        bool,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) has_presentation_uniform_payload:
        bool,
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
        CustomShaderBindingKey,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) surface_uniform_buffer:
        wgpu::Buffer,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) app_uniform_buffer:
        Option<wgpu::Buffer>,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) storage_buffer:
        Option<wgpu::Buffer>,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) presentation_uniform_buffer:
        Option<wgpu::Buffer>,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) write_state:
        CustomShaderBindingWriteState,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) bind_group:
        wgpu::BindGroup,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct CustomShaderBindingKey
{
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) pipeline_key:
        CustomShaderPipelineKey,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) uniform_bytes_len: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) storage_bytes_len: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) presentation_uniform_bytes_len:
        usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct CustomShaderBindingWriteState
{
    static_payload: Option<CustomShaderStaticPayloadKey>,
    presentation_revision: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CustomShaderStaticPayloadKey {
    storage_identity: u64,
    storage_revision: u64,
}

impl CustomShaderBindingWriteState {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn static_payload_needs_write(
        &self,
        storage_identity: u64,
        storage_revision: u64,
    ) -> bool {
        // The all-zero fence is the legacy descriptor default. Preserve its
        // historical per-draw upload behavior for callers that have not opted
        // into immutable-payload revision fencing.
        if storage_identity == 0 && storage_revision == 0 {
            return true;
        }
        self.static_payload
            != Some(CustomShaderStaticPayloadKey {
                storage_identity,
                storage_revision,
            })
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn cache_static_payload(
        &mut self,
        storage_identity: u64,
        storage_revision: u64,
    ) {
        self.static_payload = Some(CustomShaderStaticPayloadKey {
            storage_identity,
            storage_revision,
        });
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) const fn should_upload_initial_presentation(
        &self,
    ) -> bool {
        self.presentation_revision.is_none()
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn presentation_update_is_acceptable(
        &self,
        revision: u64,
        expected_byte_len: usize,
        actual_byte_len: usize,
    ) -> bool {
        expected_byte_len > 0
            && actual_byte_len == expected_byte_len
            && self
                .presentation_revision
                .is_none_or(|current| revision > current)
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn cache_presentation_revision(
        &mut self,
        revision: u64,
    ) {
        self.presentation_revision = Some(revision);
    }
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
            has_uniform_payload: false,
            has_storage_payload: false,
            has_presentation_uniform_payload: false,
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

    #[test]
    fn custom_shader_pipeline_key_tracks_payload_binding_shape() {
        let key = CustomShaderPipelineKey {
            shader_key: String::from("meter"),
            wgsl_source: Arc::<str>::from("@vertex fn vertex_main() {}"),
            vertex_entry_point: String::from("vertex_main"),
            fragment_entry_point: String::from("fragment_main"),
            has_uniform_payload: false,
            has_storage_payload: false,
            has_presentation_uniform_payload: false,
        };

        assert_ne!(
            key,
            CustomShaderPipelineKey {
                has_uniform_payload: true,
                ..key.clone()
            }
        );
        assert_ne!(
            key.clone(),
            CustomShaderPipelineKey {
                has_storage_payload: true,
                ..key
            }
        );
    }

    #[test]
    fn custom_shader_binding_write_state_caches_immutable_payload_revisions() {
        let mut state = CustomShaderBindingWriteState::default();

        assert!(state.static_payload_needs_write(7, 11));
        state.cache_static_payload(7, 11);
        assert!(!state.static_payload_needs_write(7, 11));
        assert!(state.static_payload_needs_write(7, 12));
        assert!(state.static_payload_needs_write(8, 11));
    }

    #[test]
    fn custom_shader_binding_write_state_keeps_legacy_payloads_live() {
        let mut state = CustomShaderBindingWriteState::default();

        assert!(state.static_payload_needs_write(0, 0));
        state.cache_static_payload(0, 0);
        assert!(state.static_payload_needs_write(0, 0));
    }

    #[test]
    fn custom_shader_binding_write_state_rejects_stale_presentation_revisions() {
        let mut state = CustomShaderBindingWriteState::default();

        assert!(state.should_upload_initial_presentation());
        assert!(state.presentation_update_is_acceptable(3, 4, 4));
        assert!(!state.presentation_update_is_acceptable(3, 4, 3));
        state.cache_presentation_revision(3);
        assert!(!state.should_upload_initial_presentation());
        assert!(!state.presentation_update_is_acceptable(2, 4, 4));
        assert!(!state.presentation_update_is_acceptable(3, 4, 4));
        assert!(state.presentation_update_is_acceptable(4, 4, 4));
    }
}
