use super::super::*;
use wgpu::util::DeviceExt;

mod summary;

impl GpuSurfaceRenderer {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn ensure_signal_body_texture(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        key: u64,
        body_key: SignalBodyCacheKey,
        stats: &mut GpuSurfaceRenderStats,
    ) -> Option<wgpu::TextureView> {
        if let Some(body) = self
            .resources
            .signal_bodies
            .get(&key)
            .filter(|body| body.matches_body(device, body_key))
        {
            stats.signal_body_cache_hits += 1;
            return Some(body.view.clone());
        }
        let buffer = self.resources.signals.get(&key)?;
        let pipeline = self.signal_pipeline.as_ref()?;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("radiant_gpu_signal_body_texture"),
            size: wgpu::Extent3d {
                width: body_key.width,
                height: body_key.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let started = Instant::now();
        let mut pass = signal_body_render_pass(encoder, &view);
        pass.set_scissor_rect(0, 0, body_key.width, body_key.height);
        pass.set_pipeline(&pipeline.pipeline);
        pass.set_bind_group(0, &buffer.bind_group, &[]);
        pass.draw(0..6, 0..1);
        drop(pass);
        stats.signal_body_renders += 1;
        stats.signal_body_encode_elapsed += started.elapsed();
        let cached_view = view.clone();
        self.resources.signal_bodies.insert(
            key,
            SignalBodyTexture {
                device: wgpu_device_id(device),
                cache_key: body_key,
                _texture: texture,
                view,
            },
        );
        Some(cached_view)
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn ensure_signal_buffer(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: u64,
        cache_key: SignalBufferCacheKey,
        buckets: &[GpuSignalSummaryBucket],
        uniforms: &SignalUniforms,
    ) {
        let sample_count = summary_bucket_value_count(buckets);
        if let Some(buffer) = self.resources.signals.get(&key).filter(|buffer| {
            buffer.cache_key == cache_key
                && buffer.sample_count == sample_count
                && buffer.pipeline_generation == self.signal_pipeline_generation
        }) {
            queue.write_buffer(
                &buffer.uniform_buffer,
                0,
                signal_uniforms_as_bytes(uniforms),
            );
            return;
        }
        let Some(pipeline) = self.signal_pipeline.as_ref() else {
            return;
        };
        let sample_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("radiant_gpu_signal_summary_buckets"),
            contents: summary_bucket_bytes(buckets),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("radiant_gpu_signal_uniforms"),
            contents: signal_uniforms_as_bytes(uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("radiant_gpu_signal_bind_group"),
            layout: &pipeline.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sample_buffer.as_entire_binding(),
                },
            ],
        });
        self.resources.signals.insert(
            key,
            SignalBuffer {
                cache_key,
                sample_count,
                pipeline_generation: self.signal_pipeline_generation,
                _sample_buffer: sample_buffer,
                uniform_buffer,
                bind_group,
            },
        );
    }
}
