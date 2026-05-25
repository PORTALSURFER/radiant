use super::*;

#[test]
fn custom_shader_pipeline_key_requires_source_and_fragment_entry() {
    let missing_source =
        GpuShaderSurfaceDescriptor::new("test/custom-shader").fragment_entry_point("fragment_main");
    let missing_fragment = GpuShaderSurfaceDescriptor::new("test/custom-shader").wgsl_source(
        "@vertex fn vertex_main() -> @builtin(position) vec4<f32> { return vec4<f32>(); }",
    );
    let complete = missing_fragment
        .clone()
        .fragment_entry_point("fragment_main");

    assert_eq!(custom_shader_pipeline_key(&missing_source), None);
    assert_eq!(custom_shader_pipeline_key(&missing_fragment), None);
    assert_eq!(
        custom_shader_pipeline_key(&complete).map(|key| (
            key.fragment_entry_point,
            key.has_uniform_payload,
            key.has_storage_payload,
        )),
        Some((String::from("fragment_main"), false, false))
    );
}

#[test]
fn custom_shader_pipeline_key_tracks_payload_bindings() {
    let descriptor = GpuShaderSurfaceDescriptor::new("test/custom-shader")
        .wgsl_source(
            "@vertex fn vertex_main() -> @builtin(position) vec4<f32> { return vec4<f32>(); }",
        )
        .fragment_entry_point("fragment_main")
        .uniform_bytes([1, 2, 3, 4])
        .storage_bytes([5, 6, 7, 8]);

    assert_eq!(
        custom_shader_pipeline_key(&descriptor)
            .map(|key| (key.has_uniform_payload, key.has_storage_payload,)),
        Some((true, true))
    );
}
