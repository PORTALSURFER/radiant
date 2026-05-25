use super::*;
use crate::gui_runtime::native_vello::generic_runtime::gpu_surface::gpu_surface_types::CustomShaderPipelineKey;
use std::sync::Arc;

#[test]
fn custom_shader_binding_key_tracks_payload_lengths() {
    let pipeline_key = test_pipeline_key();
    let descriptor = GpuShaderSurfaceDescriptor::new("test/custom-shader")
        .uniform_bytes([1, 2, 3, 4])
        .storage_bytes([5, 6]);

    let key = custom_shader_binding_key(&pipeline_key, &descriptor);

    assert_eq!(key.pipeline_key, pipeline_key);
    assert_eq!(key.uniform_bytes_len, 4);
    assert_eq!(key.storage_bytes_len, 2);
}

#[test]
fn custom_shader_binding_key_changes_when_payload_shape_changes() {
    let pipeline_key = test_pipeline_key();
    let uniform_only = custom_shader_binding_key(
        &pipeline_key,
        &GpuShaderSurfaceDescriptor::new("test/custom-shader").uniform_bytes([1]),
    );
    let storage_only = custom_shader_binding_key(
        &pipeline_key,
        &GpuShaderSurfaceDescriptor::new("test/custom-shader").storage_bytes([1]),
    );

    assert_ne!(uniform_only, storage_only);
}

fn test_pipeline_key() -> CustomShaderPipelineKey {
    CustomShaderPipelineKey {
        shader_key: String::from("test/custom-shader"),
        wgsl_source: Arc::<str>::from("@vertex fn vertex_main() {}"),
        vertex_entry_point: String::from("vertex_main"),
        fragment_entry_point: String::from("fragment_main"),
        has_uniform_payload: true,
        has_storage_payload: true,
    }
}
