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

#[test]
fn custom_shader_layout_entries_always_start_with_surface_uniforms() {
    let key = custom_shader_test_key(CustomShaderPayloads::None);
    let entries = layout::custom_shader_layout_entries(&key);

    assert_eq!(entries.len(), 1);
    assert_uniform_binding(&entries[0], 0);
}

#[test]
fn custom_shader_layout_entries_include_optional_payload_bindings() {
    let uniform_entries = layout::custom_shader_layout_entries(&custom_shader_test_key(
        CustomShaderPayloads::Uniform,
    ));
    let storage_entries = layout::custom_shader_layout_entries(&custom_shader_test_key(
        CustomShaderPayloads::Storage,
    ));
    let combined_entries = layout::custom_shader_layout_entries(&custom_shader_test_key(
        CustomShaderPayloads::UniformStorage,
    ));

    assert_eq!(binding_numbers(&uniform_entries), vec![0, 1]);
    assert_uniform_binding(&uniform_entries[1], 1);
    assert_eq!(binding_numbers(&storage_entries), vec![0, 2]);
    assert_storage_binding(&storage_entries[1], 2);
    assert_eq!(binding_numbers(&combined_entries), vec![0, 1, 2]);
    assert_uniform_binding(&combined_entries[1], 1);
    assert_storage_binding(&combined_entries[2], 2);
}

#[derive(Clone, Copy)]
enum CustomShaderPayloads {
    None,
    Uniform,
    Storage,
    UniformStorage,
}

impl CustomShaderPayloads {
    fn has_uniform_payload(self) -> bool {
        matches!(self, Self::Uniform | Self::UniformStorage)
    }

    fn has_storage_payload(self) -> bool {
        matches!(self, Self::Storage | Self::UniformStorage)
    }
}

fn custom_shader_test_key(payloads: CustomShaderPayloads) -> CustomShaderPipelineKey {
    CustomShaderPipelineKey {
        shader_key: String::from("test/custom-shader"),
        wgsl_source: String::from(
            "@vertex fn vertex_main() -> @builtin(position) vec4<f32> { return vec4<f32>(); }",
        )
        .into(),
        vertex_entry_point: String::from("vertex_main"),
        fragment_entry_point: String::from("fragment_main"),
        has_uniform_payload: payloads.has_uniform_payload(),
        has_storage_payload: payloads.has_storage_payload(),
    }
}

fn binding_numbers(entries: &[wgpu::BindGroupLayoutEntry]) -> Vec<u32> {
    entries.iter().map(|entry| entry.binding).collect()
}

fn assert_uniform_binding(entry: &wgpu::BindGroupLayoutEntry, binding: u32) {
    assert_eq!(entry.binding, binding);
    assert!(matches!(
        entry.ty,
        wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        }
    ));
}

fn assert_storage_binding(entry: &wgpu::BindGroupLayoutEntry, binding: u32) {
    assert_eq!(entry.binding, binding);
    assert!(matches!(
        entry.ty,
        wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        }
    ));
}
