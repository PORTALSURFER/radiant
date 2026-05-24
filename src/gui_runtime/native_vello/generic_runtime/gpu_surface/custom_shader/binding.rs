use super::super::*;
use super::diagnostics::custom_shader_validation_error;
use crate::runtime::GpuShaderSurfaceDescriptor;

impl GpuSurfaceRenderer {
    pub(super) fn ensure_custom_shader_binding(
        &mut self,
        device: &wgpu::Device,
        surface_key: u64,
        descriptor: &GpuShaderSurfaceDescriptor,
        stats: &mut GpuSurfaceRenderStats,
    ) {
        let Some(pipeline) = self.resources.custom_shader_pipelines.get(&surface_key) else {
            return;
        };
        let cache_key = CustomShaderBindingKey {
            pipeline_key: pipeline.key.clone(),
            uniform_bytes_len: descriptor.uniform_bytes.len(),
            storage_bytes_len: descriptor.storage_bytes.len(),
        };
        let rebuild = self
            .resources
            .custom_shader_bindings
            .get(&surface_key)
            .is_none_or(|binding| binding.cache_key != cache_key);
        if !rebuild {
            stats.custom_shader.binding_cache_hits += 1;
            return;
        }
        stats.custom_shader.binding_rebuilds += 1;
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let surface_uniform_buffer = custom_shader_buffer(
            device,
            Some("radiant_custom_shader_surface_uniforms"),
            std::mem::size_of::<GpuSurfaceUniforms>(),
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let app_uniform_buffer = (!descriptor.uniform_bytes.is_empty()).then(|| {
            custom_shader_buffer(
                device,
                Some("radiant_custom_shader_app_uniforms"),
                descriptor.uniform_bytes.len(),
                wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            )
        });
        let storage_buffer = (!descriptor.storage_bytes.is_empty()).then(|| {
            custom_shader_buffer(
                device,
                Some("radiant_custom_shader_storage"),
                descriptor.storage_bytes.len(),
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            )
        });
        let mut entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: surface_uniform_buffer.as_entire_binding(),
        }];
        if let Some(buffer) = &app_uniform_buffer {
            entries.push(wgpu::BindGroupEntry {
                binding: 1,
                resource: buffer.as_entire_binding(),
            });
        }
        if let Some(buffer) = &storage_buffer {
            entries.push(wgpu::BindGroupEntry {
                binding: 2,
                resource: buffer.as_entire_binding(),
            });
        }
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("radiant_custom_shader_surface_bind_group"),
            layout: &pipeline.bind_group_layout,
            entries: &entries,
        });
        if let Some(error) = custom_shader_validation_error(device) {
            stats.custom_shader.failures.binding_failures += 1;
            warn!(
                surface_key,
                shader_key = %pipeline.key.shader_key,
                uniform_bytes = descriptor.uniform_bytes.len(),
                storage_bytes = descriptor.storage_bytes.len(),
                error = %error,
                "radiant custom shader bind group validation failed"
            );
            self.resources.custom_shader_bindings.remove(&surface_key);
            return;
        }
        self.resources.custom_shader_bindings.insert(
            surface_key,
            CustomShaderBinding {
                cache_key,
                surface_uniform_buffer,
                app_uniform_buffer,
                storage_buffer,
                bind_group,
            },
        );
    }
}

fn custom_shader_buffer(
    device: &wgpu::Device,
    label: Option<&'static str>,
    size: usize,
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label,
        size: size as wgpu::BufferAddress,
        usage,
        mapped_at_creation: false,
    })
}
