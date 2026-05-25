use super::super::gpu_surface_types::{CustomShaderPipeline, CustomShaderPipelineKey};
use super::super::stats::GpuSurfaceRenderStats;
use super::super::{GpuSurfaceRenderer, wgpu_device_id};
use super::diagnostics::custom_shader_validation_error;
use crate::runtime::GpuShaderSurfaceDescriptor;
use tracing::warn;
use vello::wgpu;

pub(super) struct CustomShaderPipelineRequest<'a> {
    pub(super) surface_key: u64,
    pub(super) device: &'a wgpu::Device,
    pub(super) target_format: wgpu::TextureFormat,
    pub(super) key: CustomShaderPipelineKey,
}

struct CreatedCustomShaderPipeline {
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
}

struct CustomShaderBufferLayoutSpec {
    binding: u32,
    ty: wgpu::BufferBindingType,
}

impl GpuSurfaceRenderer {
    pub(super) fn ensure_custom_shader_pipeline(
        &mut self,
        request: CustomShaderPipelineRequest<'_>,
        stats: &mut GpuSurfaceRenderStats,
    ) {
        if !self.custom_shader_pipeline_needs_rebuild(&request) {
            return;
        }
        stats.custom_shader.pipeline_rebuilds += 1;
        self.resources
            .custom_shader_bindings
            .remove(&request.surface_key);
        let Some(shader) = create_custom_shader_module(&request, stats) else {
            self.resources
                .custom_shader_pipelines
                .remove(&request.surface_key);
            return;
        };
        let Some(created) = create_custom_shader_pipeline(&request, &shader, stats) else {
            self.resources
                .custom_shader_pipelines
                .remove(&request.surface_key);
            return;
        };
        self.resources.custom_shader_pipelines.insert(
            request.surface_key,
            CustomShaderPipeline {
                format: request.target_format,
                device: wgpu_device_id(request.device),
                key: request.key,
                bind_group_layout: created.bind_group_layout,
                pipeline: created.pipeline,
            },
        );
    }

    fn custom_shader_pipeline_needs_rebuild(
        &self,
        request: &CustomShaderPipelineRequest<'_>,
    ) -> bool {
        self.resources
            .custom_shader_pipelines
            .get(&request.surface_key)
            .is_none_or(|pipeline| {
                !pipeline.matches(request.device, request.target_format, &request.key)
            })
    }
}

fn create_custom_shader_module(
    request: &CustomShaderPipelineRequest<'_>,
    stats: &mut GpuSurfaceRenderStats,
) -> Option<wgpu::ShaderModule> {
    request
        .device
        .push_error_scope(wgpu::ErrorFilter::Validation);
    let shader = request
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("radiant_custom_shader_surface_shader"),
            source: wgpu::ShaderSource::Wgsl(request.key.wgsl_source.as_ref().into()),
        });
    if let Some(error) = custom_shader_validation_error(request.device) {
        stats.custom_shader.failures.shader_module_failures += 1;
        warn!(
            surface_key = request.surface_key,
            shader_key = %request.key.shader_key,
            error = %error,
            "radiant custom shader WGSL module validation failed"
        );
        return None;
    }
    Some(shader)
}

fn create_custom_shader_pipeline(
    request: &CustomShaderPipelineRequest<'_>,
    shader: &wgpu::ShaderModule,
    stats: &mut GpuSurfaceRenderStats,
) -> Option<CreatedCustomShaderPipeline> {
    request
        .device
        .push_error_scope(wgpu::ErrorFilter::Validation);
    let bind_group_layout = create_custom_shader_bind_group_layout(request);
    let layout = create_custom_shader_pipeline_layout(request.device, &bind_group_layout);
    let pipeline = create_custom_shader_render_pipeline(request, shader, &layout);
    if let Some(error) = custom_shader_validation_error(request.device) {
        stats.custom_shader.failures.pipeline_failures += 1;
        warn!(
            surface_key = request.surface_key,
            shader_key = %request.key.shader_key,
            vertex_entry_point = %request.key.vertex_entry_point,
            fragment_entry_point = %request.key.fragment_entry_point,
            error = %error,
            "radiant custom shader render pipeline validation failed"
        );
        return None;
    }
    Some(CreatedCustomShaderPipeline {
        bind_group_layout,
        pipeline,
    })
}

fn create_custom_shader_bind_group_layout(
    request: &CustomShaderPipelineRequest<'_>,
) -> wgpu::BindGroupLayout {
    request
        .device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("radiant_custom_shader_surface_bind_group_layout"),
            entries: &custom_shader_layout_entries(&request.key),
        })
}

fn create_custom_shader_pipeline_layout(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("radiant_custom_shader_surface_pipeline_layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    })
}

fn create_custom_shader_render_pipeline(
    request: &CustomShaderPipelineRequest<'_>,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
) -> wgpu::RenderPipeline {
    request
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("radiant_custom_shader_surface_pipeline"),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some(&request.key.vertex_entry_point),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some(&request.key.fragment_entry_point),
                targets: &[Some(wgpu::ColorTargetState {
                    format: request.target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..wgpu::PrimitiveState::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        })
}

fn custom_shader_layout_entries(key: &CustomShaderPipelineKey) -> Vec<wgpu::BindGroupLayoutEntry> {
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

pub(super) fn custom_shader_pipeline_key(
    descriptor: &GpuShaderSurfaceDescriptor,
) -> Option<CustomShaderPipelineKey> {
    Some(CustomShaderPipelineKey {
        shader_key: descriptor.shader_key.clone(),
        wgsl_source: descriptor.wgsl_source.clone()?,
        vertex_entry_point: descriptor.entry_point.clone(),
        fragment_entry_point: descriptor.fragment_entry_point.clone()?,
        has_uniform_payload: !descriptor.uniform_bytes.is_empty(),
        has_storage_payload: !descriptor.storage_bytes.is_empty(),
    })
}

#[cfg(test)]
#[path = "pipeline/tests.rs"]
mod tests;
