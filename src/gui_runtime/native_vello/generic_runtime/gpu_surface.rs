//! Native GPU renderer for retained generic GPU-surface paint primitives.

use super::*;
use crate::runtime::{
    GpuSignalSummary, GpuSignalSummaryBucket, GpuSurfaceContent, GpuSurfaceOverlay,
    PaintGpuSurface, PaintPrimitive,
};
use wgpu::util::DeviceExt;

mod encoding;
mod gpu_surface_types;
mod passes;
mod pipeline;
use encoding::*;
pub(super) use gpu_surface_types::GpuSurfaceRenderStats;
use gpu_surface_types::*;
use passes::*;
#[cfg(test)]
pub(super) use pipeline::GPU_SIGNAL_SHADER;

#[derive(Default)]
pub(super) struct GpuSurfaceRenderer {
    pipeline: Option<GpuSurfacePipeline>,
    signal_pipeline: Option<SignalPipeline>,
    signal_pipeline_generation: u64,
    textures: HashMap<u64, GpuSurfaceTexture>,
    signal_bodies: HashMap<u64, SignalBodyTexture>,
    signals: HashMap<u64, SignalBuffer>,
    signal_summaries: HashMap<u64, CachedSignalSummary>,
}

impl GpuSurfaceRenderer {
    pub(super) fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        target_format: wgpu::TextureFormat,
        target_size: Vector2,
        primitives: &[PaintPrimitive],
    ) -> GpuSurfaceRenderStats {
        let mut stats = GpuSurfaceRenderStats::default();
        for primitive in primitives {
            let PaintPrimitive::GpuSurface(surface) = primitive else {
                continue;
            };
            if surface.rect.width() <= 0.0 || surface.rect.height() <= 0.0 {
                continue;
            }
            match &surface.content {
                GpuSurfaceContent::RgbaAtlas { source_rect, atlas } => {
                    if atlas.width == 0 || atlas.height == 0 {
                        continue;
                    }
                    self.render_atlas(
                        device,
                        queue,
                        encoder,
                        target_view,
                        target_format,
                        target_size,
                        surface,
                        *source_rect,
                        &mut stats,
                    );
                }
                GpuSurfaceContent::SignalBands { samples, .. } => {
                    if samples.is_empty() {
                        continue;
                    }
                    self.render_signal(
                        device,
                        queue,
                        encoder,
                        target_view,
                        target_format,
                        target_size,
                        surface,
                        &mut stats,
                    );
                }
                GpuSurfaceContent::SignalSummaryBands { summary, .. } => {
                    if summary.levels.is_empty() {
                        continue;
                    }
                    self.render_signal(
                        device,
                        queue,
                        encoder,
                        target_view,
                        target_format,
                        target_size,
                        surface,
                        &mut stats,
                    );
                }
            }
        }
        stats
    }

    #[allow(clippy::too_many_arguments)]
    fn render_atlas(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        target_format: wgpu::TextureFormat,
        target_size: Vector2,
        surface: &PaintGpuSurface,
        source_rect: UiRect,
        stats: &mut GpuSurfaceRenderStats,
    ) {
        self.ensure_texture(device, queue, surface);
        self.ensure_pipeline(device, target_format);
        let Some(texture) = self.textures.get(&surface.key) else {
            return;
        };
        let pipeline = self.pipeline.as_ref().expect("gpu surface pipeline");
        let cursor = vertical_cursor(&surface.overlays);
        let uniforms = GpuSurfaceUniforms {
            dest: surface_dest(surface),
            source: [
                source_rect.min.x,
                source_rect.min.y,
                source_rect.width(),
                source_rect.height(),
            ],
            target_size: [target_size.x.max(1.0), target_size.y.max(1.0)],
            cursor_ratio: cursor.map(|cursor| cursor.0).unwrap_or(-1.0),
            cursor_width: cursor.map(|cursor| cursor.2).unwrap_or(1.0),
            cursor_color: cursor
                .map(|cursor| rgba_to_float(cursor.1))
                .unwrap_or([1.0, 1.0, 1.0, 0.92]),
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("radiant_gpu_surface_uniforms"),
            contents: uniforms_as_bytes(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("radiant_gpu_surface_bind_group"),
            layout: &pipeline.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&pipeline.sampler),
                },
            ],
        });
        let started = Instant::now();
        let mut pass = gpu_surface_render_pass(encoder, target_view);
        set_surface_scissor(&mut pass, surface.rect);
        pass.set_pipeline(&pipeline.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..6, 0..1);
        stats.composite_encode_elapsed += started.elapsed();
    }

    #[allow(clippy::too_many_arguments)]
    fn render_signal(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        target_format: wgpu::TextureFormat,
        target_size: Vector2,
        surface: &PaintGpuSurface,
        stats: &mut GpuSurfaceRenderStats,
    ) {
        let (frames, band_count, frame_range, summary) = match &surface.content {
            GpuSurfaceContent::SignalBands {
                frames,
                band_count,
                frame_range,
                samples,
            } => {
                let summary = self.cached_signal_summary(
                    surface.key,
                    surface.revision,
                    *frames,
                    *band_count,
                    samples,
                );
                (*frames, *band_count, *frame_range, summary)
            }
            GpuSurfaceContent::SignalSummaryBands {
                frames,
                band_count,
                frame_range,
                summary,
            } => (*frames, *band_count, *frame_range, Arc::clone(summary)),
            _ => return,
        };
        let visible = (frame_range[1] - frame_range[0]).max(1.0);
        let frames_per_pixel = visible / surface.rect.width().max(1.0);
        let level_index = summary.level_for_frames_per_pixel(frames_per_pixel);
        let Some(level) = summary.levels.get(level_index) else {
            return;
        };
        let body_key = SignalBodyCacheKey::new(
            surface,
            frames,
            band_count,
            frame_range,
            level.buckets.len(),
            level_index,
        );
        self.ensure_pipeline(device, target_format);
        if self
            .signal_bodies
            .get(&surface.key)
            .is_some_and(|body| body.cache_key == body_key)
        {
            stats.signal_body_cache_hits += 1;
            let Some(body) = self.signal_bodies.get(&surface.key) else {
                return;
            };
            self.render_texture_view(
                device,
                encoder,
                target_view,
                target_format,
                target_size,
                surface,
                &body.view,
                [0.0, 0.0, body_key.width as f32, body_key.height as f32],
                stats,
            );
            return;
        }
        self.ensure_signal_pipeline(device, wgpu::TextureFormat::Rgba8Unorm);
        let uniforms = SignalUniforms {
            dest: [0.0, 0.0, body_key.width as f32, body_key.height as f32],
            frame_range: [
                frame_range[0],
                frame_range[1],
                frames as f32,
                band_count as f32,
            ],
            summary_meta: [
                level.bucket_frames as f32,
                (level.buckets.len() / band_count.max(1)) as f32,
                level_index as f32,
                0.0,
            ],
            target_size: [body_key.width as f32, body_key.height as f32],
            cursor_ratio: -1.0,
            cursor_width: 1.0,
            cursor_color: [1.0, 1.0, 1.0, 0.92],
        };
        self.ensure_signal_buffer(
            device,
            queue,
            surface.key,
            surface.revision ^ ((level_index as u64) << 32),
            level.buckets.as_ref(),
            &uniforms,
        );
        self.ensure_signal_body_texture(device, encoder, surface.key, body_key, stats);
        let Some(body) = self.signal_bodies.get(&surface.key) else {
            return;
        };
        self.render_texture_view(
            device,
            encoder,
            target_view,
            target_format,
            target_size,
            surface,
            &body.view,
            [0.0, 0.0, body_key.width as f32, body_key.height as f32],
            stats,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn render_texture_view(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        target_format: wgpu::TextureFormat,
        target_size: Vector2,
        surface: &PaintGpuSurface,
        texture_view: &wgpu::TextureView,
        source: [f32; 4],
        stats: &mut GpuSurfaceRenderStats,
    ) {
        let _ = (device, target_format);
        let Some(pipeline) = self.pipeline.as_ref() else {
            return;
        };
        let cursor = vertical_cursor(&surface.overlays);
        let uniforms = GpuSurfaceUniforms {
            dest: surface_dest(surface),
            source,
            target_size: [target_size.x.max(1.0), target_size.y.max(1.0)],
            cursor_ratio: cursor.map(|cursor| cursor.0).unwrap_or(-1.0),
            cursor_width: cursor.map(|cursor| cursor.2).unwrap_or(1.0),
            cursor_color: cursor
                .map(|cursor| rgba_to_float(cursor.1))
                .unwrap_or([1.0, 1.0, 1.0, 0.92]),
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("radiant_gpu_surface_uniforms"),
            contents: uniforms_as_bytes(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("radiant_gpu_surface_bind_group"),
            layout: &pipeline.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&pipeline.sampler),
                },
            ],
        });
        let started = Instant::now();
        let mut pass = gpu_surface_render_pass(encoder, target_view);
        set_surface_scissor(&mut pass, surface.rect);
        pass.set_pipeline(&pipeline.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..6, 0..1);
        stats.composite_encode_elapsed += started.elapsed();
    }

    fn ensure_signal_body_texture(
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
        let pipeline = self
            .signal_pipeline
            .as_ref()
            .expect("gpu signal surface pipeline");
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

    fn ensure_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface: &PaintGpuSurface,
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
    }

    fn ensure_pipeline(&mut self, device: &wgpu::Device, target_format: wgpu::TextureFormat) {
        let rebuild = self
            .pipeline
            .as_ref()
            .is_none_or(|pipeline| pipeline.format != target_format);
        if rebuild {
            self.pipeline = Some(GpuSurfacePipeline::new(device, target_format));
        }
    }

    fn ensure_signal_buffer(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: u64,
        revision: u64,
        buckets: &[GpuSignalSummaryBucket],
        uniforms: &SignalUniforms,
    ) {
        let values = summary_buckets_as_f32s(buckets);
        if self.signals.get(&key).is_some_and(|buffer| {
            buffer.revision == revision
                && buffer.sample_count == values.len()
                && buffer.pipeline_generation == self.signal_pipeline_generation
        }) {
            let buffer = self.signals.get(&key).expect("checked signal buffer");
            queue.write_buffer(
                &buffer.uniform_buffer,
                0,
                signal_uniforms_as_bytes(uniforms),
            );
            return;
        }
        let pipeline = self
            .signal_pipeline
            .as_ref()
            .expect("gpu signal surface pipeline");
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

    fn cached_signal_summary(
        &mut self,
        key: u64,
        revision: u64,
        frames: usize,
        band_count: usize,
        samples: &Arc<[f32]>,
    ) -> Arc<GpuSignalSummary> {
        if let Some(cached) = self.signal_summaries.get(&key)
            && cached.revision == revision
            && cached.frames == frames
            && cached.band_count == band_count
            && cached.sample_count == samples.len()
        {
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
        summary
    }

    fn ensure_signal_pipeline(
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
