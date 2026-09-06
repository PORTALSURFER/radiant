use std::sync::Arc;
use vello::wgpu;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::gui_runtime::native_vello::generic_runtime) struct CustomShaderPipelineKey {
    pub(in crate::gui_runtime::native_vello::generic_runtime) shader_key: Arc<str>,
    pub(in crate::gui_runtime::native_vello::generic_runtime) wgsl_source: Arc<str>,
    pub(in crate::gui_runtime::native_vello::generic_runtime) vertex_entry_point: Arc<str>,
    pub(in crate::gui_runtime::native_vello::generic_runtime) fragment_entry_point: Arc<str>,
    pub(in crate::gui_runtime::native_vello::generic_runtime) has_uniform_payload: bool,
    pub(in crate::gui_runtime::native_vello::generic_runtime) has_storage_payload: bool,
    pub(in crate::gui_runtime::native_vello::generic_runtime) has_presentation_uniform_payload:
        bool,
}

impl CustomShaderPipelineKey {
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn text_bytes(&self) -> usize {
        self.shader_key
            .len()
            .saturating_add(self.wgsl_source.len())
            .saturating_add(self.vertex_entry_point.len())
            .saturating_add(self.fragment_entry_point.len())
    }
}

/// The complete physical-pipeline identity.  Surface payload and revisions are
/// deliberately absent: those are owned by the per-surface binding cache.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::gui_runtime::native_vello::generic_runtime) struct CustomShaderPipelineIdentity {
    pub(in crate::gui_runtime::native_vello::generic_runtime) device: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime) format: wgpu::TextureFormat,
    pub(in crate::gui_runtime::native_vello::generic_runtime) key: CustomShaderPipelineKey,
}

#[derive(Clone)]
pub(in crate::gui_runtime::native_vello::generic_runtime) struct CustomShaderPipeline {
    pub(in crate::gui_runtime::native_vello::generic_runtime) key: CustomShaderPipelineKey,
    pub(in crate::gui_runtime::native_vello::generic_runtime) bind_group_layout:
        wgpu::BindGroupLayout,
    pub(in crate::gui_runtime::native_vello::generic_runtime) pipeline: wgpu::RenderPipeline,
}

#[derive(Clone)]
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
    presentation_static_payload: Option<CustomShaderStaticPayloadKey>,
    presentation_revision: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct CustomShaderStaticPayloadKey
{
    storage_identity: u64,
    storage_revision: u64,
    uniform_bytes_len: usize,
    storage_bytes_len: usize,
}

impl CustomShaderStaticPayloadKey {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) const fn new(
        storage_identity: u64,
        storage_revision: u64,
        uniform_bytes_len: usize,
        storage_bytes_len: usize,
    ) -> Self {
        Self {
            storage_identity,
            storage_revision,
            uniform_bytes_len,
            storage_bytes_len,
        }
    }
}

impl CustomShaderBindingWriteState {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn static_payload_needs_write(
        &self,
        static_payload: CustomShaderStaticPayloadKey,
    ) -> bool {
        // The all-zero fence is the legacy descriptor default. Preserve its
        // historical per-draw upload behavior for callers that have not opted
        // into immutable-payload revision fencing.
        if static_payload.storage_identity == 0 && static_payload.storage_revision == 0 {
            return true;
        }
        self.static_payload != Some(static_payload)
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn cache_static_payload(
        &mut self,
        static_payload: CustomShaderStaticPayloadKey,
    ) {
        self.static_payload = Some(static_payload);
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn should_upload_initial_presentation(
        &self,
        static_payload: CustomShaderStaticPayloadKey,
        revision: u64,
    ) -> bool {
        self.presentation_static_payload != Some(static_payload)
            || self
                .presentation_revision
                .is_none_or(|current| revision > current)
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn presentation_update_is_acceptable(
        &self,
        static_payload: CustomShaderStaticPayloadKey,
        revision: u64,
        expected_byte_len: usize,
        actual_byte_len: usize,
    ) -> bool {
        expected_byte_len > 0
            && actual_byte_len == expected_byte_len
            && (self.presentation_static_payload != Some(static_payload)
                || self
                    .presentation_revision
                    .is_none_or(|current| revision > current))
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn cache_presentation_revision(
        &mut self,
        static_payload: CustomShaderStaticPayloadKey,
        revision: u64,
    ) {
        self.presentation_static_payload = Some(static_payload);
        self.presentation_revision = Some(revision);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_shader_pipeline_key_tracks_shader_stage_contract() {
        let key = CustomShaderPipelineKey {
            shader_key: Arc::from("meter"),
            wgsl_source: Arc::<str>::from("@vertex fn vertex_main() {}"),
            vertex_entry_point: Arc::from("vertex_main"),
            fragment_entry_point: Arc::from("fragment_main"),
            has_uniform_payload: false,
            has_storage_payload: false,
            has_presentation_uniform_payload: false,
        };

        assert_ne!(
            key,
            CustomShaderPipelineKey {
                fragment_entry_point: Arc::from("other_fragment"),
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
            shader_key: Arc::from("meter"),
            wgsl_source: Arc::<str>::from("@vertex fn vertex_main() {}"),
            vertex_entry_point: Arc::from("vertex_main"),
            fragment_entry_point: Arc::from("fragment_main"),
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
        let payload = CustomShaderStaticPayloadKey::new(7, 11, 4, 8);
        let next_revision = CustomShaderStaticPayloadKey::new(7, 12, 4, 8);
        let next_identity = CustomShaderStaticPayloadKey::new(8, 11, 4, 8);

        assert!(state.static_payload_needs_write(payload));
        state.cache_static_payload(payload);
        assert!(!state.static_payload_needs_write(payload));
        assert!(state.static_payload_needs_write(next_revision));
        assert!(state.static_payload_needs_write(next_identity));
    }

    #[test]
    fn custom_shader_binding_write_state_keeps_legacy_payloads_live() {
        let mut state = CustomShaderBindingWriteState::default();
        let payload = CustomShaderStaticPayloadKey::new(0, 0, 4, 8);

        assert!(state.static_payload_needs_write(payload));
        state.cache_static_payload(payload);
        assert!(state.static_payload_needs_write(payload));
    }

    #[test]
    fn custom_shader_binding_write_state_rejects_stale_descriptor_after_newer_mailbox() {
        let mut state = CustomShaderBindingWriteState::default();
        let payload = CustomShaderStaticPayloadKey::new(7, 11, 4, 8);

        assert!(state.should_upload_initial_presentation(payload, 3));
        assert!(state.presentation_update_is_acceptable(payload, 3, 4, 4));
        assert!(!state.presentation_update_is_acceptable(payload, 3, 4, 3));
        // A newer mailbox update has already been written for this generation.
        state.cache_presentation_revision(payload, 3);
        assert!(!state.should_upload_initial_presentation(payload, 2));
        assert!(!state.should_upload_initial_presentation(payload, 3));
        assert!(!state.presentation_update_is_acceptable(payload, 2, 4, 4));
        assert!(!state.presentation_update_is_acceptable(payload, 3, 4, 4));
        assert!(state.presentation_update_is_acceptable(payload, 4, 4, 4));
    }

    #[test]
    fn custom_shader_binding_write_state_accepts_newer_descriptor_revisions() {
        let mut state = CustomShaderBindingWriteState::default();
        let payload = CustomShaderStaticPayloadKey::new(7, 11, 4, 8);

        state.cache_presentation_revision(payload, 3);
        assert!(state.should_upload_initial_presentation(payload, 4));
        state.cache_presentation_revision(payload, 4);
        assert!(!state.should_upload_initial_presentation(payload, 4));
    }

    #[test]
    fn custom_shader_binding_write_state_resets_presentation_for_new_static_generation() {
        let mut state = CustomShaderBindingWriteState::default();
        let first_generation = CustomShaderStaticPayloadKey::new(7, 11, 4, 8);
        let next_generation = CustomShaderStaticPayloadKey::new(8, 2, 4, 8);

        state.cache_presentation_revision(first_generation, 9);
        assert!(!state.should_upload_initial_presentation(first_generation, 9));
        assert!(!state.presentation_update_is_acceptable(first_generation, 8, 4, 4));

        // A same-shape storage generation may restart its volatile revision.
        assert!(state.should_upload_initial_presentation(next_generation, 1));
        assert!(state.presentation_update_is_acceptable(next_generation, 1, 4, 4));
        state.cache_presentation_revision(next_generation, 1);
        assert!(!state.presentation_update_is_acceptable(next_generation, 1, 4, 4));
    }
}
