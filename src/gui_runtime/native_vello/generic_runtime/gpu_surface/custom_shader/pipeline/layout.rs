use super::super::super::gpu_surface_types::CustomShaderPipelineKey;
use super::CustomShaderPipelineRequest;
use vello::wgpu;

struct CustomShaderBufferLayoutSpec {
    binding: u32,
    ty: wgpu::BufferBindingType,
}

pub(super) fn create_custom_shader_bind_group_layout(
    request: &CustomShaderPipelineRequest<'_>,
) -> wgpu::BindGroupLayout {
    request
        .device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("radiant_custom_shader_surface_bind_group_layout"),
            entries: &custom_shader_layout_entries(&request.key),
        })
}

pub(super) fn create_custom_shader_pipeline_layout(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("radiant_custom_shader_surface_pipeline_layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    })
}

pub(super) fn custom_shader_layout_entries(
    key: &CustomShaderPipelineKey,
) -> Vec<wgpu::BindGroupLayoutEntry> {
    let mut entries = vec![custom_shader_buffer_layout_entry(
        CustomShaderBufferLayoutSpec {
            binding: 0,
            ty: wgpu::BufferBindingType::Uniform,
        },
    )];
    if key.has_uniform_payload {
        entries.push(custom_shader_buffer_layout_entry(
            CustomShaderBufferLayoutSpec {
                binding: 1,
                ty: wgpu::BufferBindingType::Uniform,
            },
        ));
    }
    if key.has_storage_payload {
        entries.push(custom_shader_buffer_layout_entry(
            CustomShaderBufferLayoutSpec {
                binding: 2,
                ty: wgpu::BufferBindingType::Storage { read_only: true },
            },
        ));
    }
    entries
}

fn custom_shader_buffer_layout_entry(
    spec: CustomShaderBufferLayoutSpec,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: spec.binding,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: spec.ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}
