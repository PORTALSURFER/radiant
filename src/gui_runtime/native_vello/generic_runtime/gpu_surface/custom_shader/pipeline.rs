use super::super::gpu_surface_types::{CustomShaderPipeline, CustomShaderPipelineKey};
use super::super::stats::GpuSurfaceRenderStats;
use super::super::{GpuSurfaceRenderer, wgpu_device_id};
use super::diagnostics::custom_shader_validation_error;
use crate::runtime::GpuShaderSurfaceDescriptor;
use tracing::warn;
use vello::wgpu;

impl GpuSurfaceRenderer {
    pub(super) fn ensure_custom_shader_pipeline(
        &mut self,
        surface_key: u64,
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
        key: CustomShaderPipelineKey,
        stats: &mut GpuSurfaceRenderStats,
    ) {
        let rebuild = self
            .resources
            .custom_shader_pipelines
            .get(&surface_key)
            .is_none_or(|pipeline| !pipeline.matches(device, target_format, &key));
        if !rebuild {
            return;
        }
        stats.custom_shader.pipeline_rebuilds += 1;
        self.resources.custom_shader_bindings.remove(&surface_key);
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("radiant_custom_shader_surface_shader"),
            source: wgpu::ShaderSource::Wgsl(key.wgsl_source.as_ref().into()),
        });
        if let Some(error) = custom_shader_validation_error(device) {
            stats.custom_shader.failures.shader_module_failures += 1;
            warn!(
                surface_key,
                shader_key = %key.shader_key,
                error = %error,
                "radiant custom shader WGSL module validation failed"
            );
            self.resources.custom_shader_pipelines.remove(&surface_key);
            return;
        }
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("radiant_custom_shader_surface_bind_group_layout"),
            entries: &custom_shader_layout_entries(&key),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("radiant_custom_shader_surface_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("radiant_custom_shader_surface_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some(&key.vertex_entry_point),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some(&key.fragment_entry_point),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
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
        });
        if let Some(error) = custom_shader_validation_error(device) {
            stats.custom_shader.failures.pipeline_failures += 1;
            warn!(
                surface_key,
                shader_key = %key.shader_key,
                vertex_entry_point = %key.vertex_entry_point,
                fragment_entry_point = %key.fragment_entry_point,
                error = %error,
                "radiant custom shader render pipeline validation failed"
            );
            self.resources.custom_shader_pipelines.remove(&surface_key);
            return;
        }
        self.resources.custom_shader_pipelines.insert(
            surface_key,
            CustomShaderPipeline {
                format: target_format,
                device: wgpu_device_id(device),
                key,
                bind_group_layout,
                pipeline,
            },
        );
    }
}

fn custom_shader_layout_entries(key: &CustomShaderPipelineKey) -> Vec<wgpu::BindGroupLayoutEntry> {
    let mut entries = vec![wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }];
    if key.has_uniform_payload {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
    }
    if key.has_storage_payload {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
    }
    entries
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
mod tests {
    use super::*;

    #[test]
    fn custom_shader_pipeline_key_requires_source_and_fragment_entry() {
        let missing_source = GpuShaderSurfaceDescriptor::new("test/custom-shader")
            .fragment_entry_point("fragment_main");
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
}
