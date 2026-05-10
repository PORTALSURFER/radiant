use super::*;
use wgpu::util::DeviceExt;

impl GpuSurfaceRenderer {
    pub(super) fn ensure_signal_body_texture(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        key: u64,
        body_key: SignalBodyCacheKey,
        stats: &mut GpuSurfaceRenderStats,
    ) {
        if self
            .signal_bodies
            .get(&key)
            .is_some_and(|body| body.cache_key == body_key)
        {
            stats.signal_body_cache_hits += 1;
            return;
        }
        let Some(buffer) = self.signals.get(&key) else {
            return;
        };
        let Some(pipeline) = self.signal_pipeline.as_ref() else {
            return;
        };
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
        self.signal_bodies.insert(
            key,
            SignalBodyTexture {
                cache_key: body_key,
                _texture: texture,
                view,
            },
        );
    }

    pub(super) fn ensure_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface: &PaintGpuSurface,
        stats: &mut GpuSurfaceRenderStats,
    ) {
        let GpuSurfaceContent::RgbaAtlas { atlas, .. } = &surface.content else {
            return;
        };
        if self.textures.get(&surface.key).is_some_and(|texture| {
            texture.revision == surface.revision
                && texture.width == atlas.width
                && texture.height == atlas.height
        }) {
            return;
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("radiant_gpu_surface_texture"),
            size: wgpu::Extent3d {
                width: atlas.width as u32,
                height: atlas.height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            atlas.pixels.as_ref(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some((atlas.width * 4) as u32),
                rows_per_image: Some(atlas.height as u32),
            },
            wgpu::Extent3d {
                width: atlas.width as u32,
                height: atlas.height as u32,
                depth_or_array_layers: 1,
            },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.textures.insert(
            surface.key,
            GpuSurfaceTexture {
                revision: surface.revision,
                width: atlas.width,
                height: atlas.height,
                _texture: texture,
                view,
            },
        );
        stats.atlas_texture_uploads += 1;
    }

    pub(super) fn ensure_pipeline(
        &mut self,
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
    ) {
        let rebuild = self
            .pipeline
            .as_ref()
            .is_none_or(|pipeline| pipeline.format != target_format);
        if rebuild {
            self.pipeline = Some(GpuSurfacePipeline::new(device, target_format));
        }
    }

    pub(super) fn ensure_signal_buffer(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: u64,
        revision: u64,
        buckets: &[GpuSignalSummaryBucket],
        uniforms: &SignalUniforms,
    ) {
        let sample_count = summary_bucket_value_count(buckets);
        if let Some(buffer) = self.signals.get(&key).filter(|buffer| {
            buffer.revision == revision
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
        let values = summary_buckets_as_f32s(buckets);
        let sample_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("radiant_gpu_signal_summary_buckets"),
            contents: summary_bucket_bytes(&values),
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
        self.signals.insert(
            key,
            SignalBuffer {
                revision,
                sample_count: values.len(),
                pipeline_generation: self.signal_pipeline_generation,
                _sample_buffer: sample_buffer,
                uniform_buffer,
                bind_group,
            },
        );
    }

    pub(super) fn cached_signal_summary(
        &mut self,
        key: u64,
        revision: u64,
        frames: usize,
        band_count: usize,
        samples: &Arc<[f32]>,
        stats: &mut GpuSurfaceRenderStats,
    ) -> Arc<GpuSignalSummary> {
        if let Some(cached) = self.signal_summaries.get(&key)
            && cached.revision == revision
            && cached.frames == frames
            && cached.band_count == band_count
            && cached.sample_count == samples.len()
        {
            stats.signal_summary_cache_hits += 1;
            return Arc::clone(&cached.summary);
        }
        let summary = Arc::new(GpuSignalSummary::from_interleaved_samples(
            samples, frames, band_count,
        ));
        self.signal_summaries.insert(
            key,
            CachedSignalSummary {
                revision,
                frames,
                band_count,
                sample_count: samples.len(),
                summary: Arc::clone(&summary),
            },
        );
        stats.signal_summary_builds += 1;
        summary
    }

    pub(super) fn ensure_signal_pipeline(
        &mut self,
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
    ) {
        let rebuild = self
            .signal_pipeline
            .as_ref()
            .is_none_or(|pipeline| pipeline.format != target_format);
        if rebuild {
            self.signal_pipeline = Some(SignalPipeline::new(device, target_format));
            self.signal_pipeline_generation = self.signal_pipeline_generation.wrapping_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_signal_summary_reports_builds_and_hits() {
        let mut renderer = GpuSurfaceRenderer::default();
        let samples: Arc<[f32]> = [-0.5, 0.25, 0.75, -0.25].into_iter().collect();
        let mut stats = GpuSurfaceRenderStats::default();

        let first = renderer.cached_signal_summary(7, 1, 4, 1, &samples, &mut stats);

        assert_eq!(stats.signal_summary_builds, 1);
        assert_eq!(stats.signal_summary_cache_hits, 0);

        let second = renderer.cached_signal_summary(7, 1, 4, 1, &samples, &mut stats);

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(stats.signal_summary_builds, 1);
        assert_eq!(stats.signal_summary_cache_hits, 1);
    }

    #[test]
    fn cached_signal_summary_rebuilds_when_source_shape_changes() {
        let mut renderer = GpuSurfaceRenderer::default();
        let samples: Arc<[f32]> = [-0.5, 0.25, 0.75, -0.25].into_iter().collect();
        let mut stats = GpuSurfaceRenderStats::default();

        let first = renderer.cached_signal_summary(7, 1, 4, 1, &samples, &mut stats);
        let second = renderer.cached_signal_summary(7, 1, 2, 2, &samples, &mut stats);

        assert!(!Arc::ptr_eq(&first, &second));
        assert_eq!(stats.signal_summary_builds, 2);
        assert_eq!(stats.signal_summary_cache_hits, 0);
    }
}
