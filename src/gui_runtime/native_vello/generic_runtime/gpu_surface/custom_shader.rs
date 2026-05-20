use crate::runtime::{GpuShaderSurfaceDescriptor, PaintGpuSurface};

use super::*;

impl GpuSurfaceRenderer {
    pub(super) fn render_custom_shader(
        &mut self,
        target: &mut GpuSurfaceRenderTarget<'_>,
        surface: &PaintGpuSurface,
        occlusion_regions: &[UiRect],
        stats: &mut GpuSurfaceRenderStats,
    ) {
        let crate::runtime::GpuSurfaceContent::CustomShader { descriptor } = &surface.content
        else {
            return;
        };
        if descriptor.wgsl_source.is_none() || descriptor.fragment_entry_point.is_none() {
            record_unsupported_custom_shader(descriptor, stats);
            return;
        }

        let Some(pipeline_key) = custom_shader_pipeline_key(descriptor) else {
            record_unsupported_custom_shader(descriptor, stats);
            return;
        };
        self.ensure_custom_shader_pipeline(
            surface.key,
            target.device,
            target.format,
            pipeline_key,
            stats,
        );
        if !self
            .resources
            .custom_shader_pipelines
            .contains_key(&surface.key)
        {
            record_failed_custom_shader_surface(stats);
            return;
        }
        self.ensure_custom_shader_binding(target.device, surface.key, descriptor, stats);
        let Some(pipeline) = self.resources.custom_shader_pipelines.get(&surface.key) else {
            record_failed_custom_shader_surface(stats);
            return;
        };
        let Some(binding) = self.resources.custom_shader_bindings.get(&surface.key) else {
            record_failed_custom_shader_surface(stats);
            return;
        };
        let uniforms = GpuSurfaceUniforms {
            dest: surface_dest(surface),
            target_size: [target.size.x.max(1.0), target.size.y.max(1.0)],
            ..GpuSurfaceUniforms::default()
        };
        target.queue.write_buffer(
            &binding.surface_uniform_buffer,
            0,
            uniforms_as_bytes(&uniforms),
        );
        if let Some(buffer) = &binding.app_uniform_buffer {
            target
                .queue
                .write_buffer(buffer, 0, &descriptor.uniform_bytes);
        }
        if let Some(buffer) = &binding.storage_buffer {
            target
                .queue
                .write_buffer(buffer, 0, &descriptor.storage_bytes);
        }
        let started = Instant::now();
        let mut pass = gpu_surface_render_pass(target.encoder, target.target_view);
        pass.set_pipeline(&pipeline.pipeline);
        pass.set_bind_group(0, &binding.bind_group, &[]);
        for region in visible_surface_regions(surface.rect, occlusion_regions) {
            if set_surface_scissor(&mut pass, region) {
                pass.draw(0..descriptor.vertex_count, 0..1);
            }
        }
        stats.custom_shader_surfaces_rendered += 1;
        stats.composite_encode_elapsed += started.elapsed();
    }

    fn ensure_custom_shader_pipeline(
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
        stats.custom_shader_pipeline_rebuilds += 1;
        self.resources.custom_shader_bindings.remove(&surface_key);
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("radiant_custom_shader_surface_shader"),
            source: wgpu::ShaderSource::Wgsl(key.wgsl_source.as_ref().into()),
        });
        if let Some(error) = custom_shader_validation_error(device) {
            stats.custom_shader_shader_module_failures += 1;
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
            stats.custom_shader_pipeline_failures += 1;
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

    fn ensure_custom_shader_binding(
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
            stats.custom_shader_binding_cache_hits += 1;
            return;
        }
        stats.custom_shader_binding_rebuilds += 1;
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
            stats.custom_shader_binding_failures += 1;
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

fn custom_shader_validation_error(device: &wgpu::Device) -> Option<wgpu::Error> {
    pollster::block_on(device.pop_error_scope())
}

fn custom_shader_pipeline_key(
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

fn record_unsupported_custom_shader(
    descriptor: &GpuShaderSurfaceDescriptor,
    stats: &mut GpuSurfaceRenderStats,
) {
    stats.unsupported_custom_shader_surfaces += 1;
    stats.unsupported_custom_shader_vertices += descriptor.vertex_count as usize;
    stats.unsupported_custom_shader_source_bytes += descriptor
        .wgsl_source
        .as_ref()
        .map_or(0, |source| source.len());
    stats.unsupported_custom_shader_uniform_bytes += descriptor.uniform_bytes.len();
    stats.unsupported_custom_shader_storage_bytes += descriptor.storage_bytes.len();
}

fn record_failed_custom_shader_surface(stats: &mut GpuSurfaceRenderStats) {
    stats.custom_shader_surfaces_failed += 1;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        layout::{Point, Rect, Vector2},
        runtime::{GpuShaderSurfaceDescriptor, GpuSurfaceCapabilities, GpuSurfaceContent},
    };
    use std::sync::Arc;

    #[test]
    fn custom_shader_unsupported_diagnostics_count_payload_bytes() {
        let mut stats = GpuSurfaceRenderStats::default();
        let surface = PaintGpuSurface {
            widget_id: 17,
            key: 93,
            revision: 2,
            rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 24.0)),
            content: GpuSurfaceContent::CustomShader {
                descriptor: Arc::new(
                    GpuShaderSurfaceDescriptor::new("test/custom-shader")
                        .wgsl_source(
                            "@vertex fn vertex_main() -> @builtin(position) vec4<f32> { return vec4<f32>(); }\n@fragment fn fragment_main() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }",
                        )
                        .entry_point("vertex_main")
                        .fragment_entry_point("fragment_main")
                        .uniform_bytes([1, 2, 3, 4])
                        .storage_bytes([5, 6, 7])
                        .vertex_count(6),
                ),
            },
            capabilities: GpuSurfaceCapabilities::default(),
            overlays: Vec::new(),
        };

        record_unsupported_custom_shader(
            match &surface.content {
                GpuSurfaceContent::CustomShader { descriptor } => descriptor,
                _ => unreachable!(),
            },
            &mut stats,
        );

        assert_eq!(stats.unsupported_custom_shader_surfaces, 1);
        assert_eq!(stats.unsupported_custom_shader_vertices, 6);
        assert!(stats.unsupported_custom_shader_source_bytes > 0);
        assert_eq!(stats.unsupported_custom_shader_uniform_bytes, 4);
        assert_eq!(stats.unsupported_custom_shader_storage_bytes, 3);
    }

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
