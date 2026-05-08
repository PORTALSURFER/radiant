//! Native GPU renderer for retained generic GPU-surface paint primitives.

use super::*;
use crate::runtime::{GpuSurfaceContent, GpuSurfaceOverlay, PaintGpuSurface, PaintPrimitive};
use wgpu::util::DeviceExt;

#[derive(Default)]
pub(super) struct GpuSurfaceRenderer {
    pipeline: Option<GpuSurfacePipeline>,
    signal_pipeline: Option<SignalPipeline>,
    signal_pipeline_generation: u64,
    textures: HashMap<u64, GpuSurfaceTexture>,
    signal_bodies: HashMap<u64, SignalBodyTexture>,
    signals: HashMap<u64, SignalBuffer>,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct GpuSurfaceRenderStats {
    pub(super) signal_body_renders: usize,
    pub(super) signal_body_cache_hits: usize,
    pub(super) signal_body_encode_elapsed: Duration,
    pub(super) composite_encode_elapsed: Duration,
}

struct GpuSurfacePipeline {
    format: wgpu::TextureFormat,
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
}

struct GpuSurfaceTexture {
    revision: u64,
    width: usize,
    height: usize,
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

struct SignalPipeline {
    format: wgpu::TextureFormat,
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
}

struct SignalBuffer {
    revision: u64,
    sample_count: usize,
    pipeline_generation: u64,
    _sample_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

struct SignalBodyTexture {
    cache_key: SignalBodyCacheKey,
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SignalBodyCacheKey {
    revision: u64,
    width: u32,
    height: u32,
    frame_start_bits: u32,
    frame_end_bits: u32,
    frames: usize,
    band_count: usize,
    sample_count: usize,
}

impl SignalBodyCacheKey {
    fn new(
        surface: &PaintGpuSurface,
        frames: usize,
        band_count: usize,
        frame_range: [f32; 2],
        sample_count: usize,
    ) -> Self {
        Self {
            revision: surface.revision,
            width: surface.rect.width().ceil().max(1.0) as u32,
            height: surface.rect.height().ceil().max(1.0) as u32,
            frame_start_bits: frame_range[0].to_bits(),
            frame_end_bits: frame_range[1].to_bits(),
            frames,
            band_count,
            sample_count,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct GpuSurfaceUniforms {
    dest: [f32; 4],
    source: [f32; 4],
    target_size: [f32; 2],
    cursor_ratio: f32,
    cursor_width: f32,
    cursor_color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct SignalUniforms {
    dest: [f32; 4],
    frame_range: [f32; 4],
    target_size: [f32; 2],
    cursor_ratio: f32,
    cursor_width: f32,
    cursor_color: [f32; 4],
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
        let GpuSurfaceContent::SignalBands {
            frames,
            band_count,
            frame_range,
            samples,
        } = &surface.content
        else {
            return;
        };
        self.ensure_signal_pipeline(device, wgpu::TextureFormat::Rgba8Unorm);
        let body_key =
            SignalBodyCacheKey::new(surface, *frames, *band_count, *frame_range, samples.len());
        let uniforms = SignalUniforms {
            dest: [0.0, 0.0, body_key.width as f32, body_key.height as f32],
            frame_range: [
                frame_range[0],
                frame_range[1],
                *frames as f32,
                *band_count as f32,
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
            surface.revision,
            samples,
            &uniforms,
        );
        self.ensure_pipeline(device, target_format);
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
        samples: &Arc<[f32]>,
        uniforms: &SignalUniforms,
    ) {
        if self.signals.get(&key).is_some_and(|buffer| {
            buffer.revision == revision
                && buffer.sample_count == samples.len()
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
            label: Some("radiant_gpu_signal_samples"),
            contents: f32s_as_bytes(samples),
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
                sample_count: samples.len(),
                pipeline_generation: self.signal_pipeline_generation,
                _sample_buffer: sample_buffer,
                uniform_buffer,
                bind_group,
            },
        );
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

impl GpuSurfacePipeline {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("radiant_gpu_surface_shader"),
            source: wgpu::ShaderSource::Wgsl(GPU_SURFACE_SHADER.into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("radiant_gpu_surface_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("radiant_gpu_surface_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("radiant_gpu_surface_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
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
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("radiant_gpu_surface_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });
        Self {
            format,
            bind_group_layout,
            pipeline,
            sampler,
        }
    }
}

impl SignalPipeline {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("radiant_gpu_signal_surface_shader"),
            source: wgpu::ShaderSource::Wgsl(GPU_SIGNAL_SHADER.into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("radiant_gpu_signal_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("radiant_gpu_signal_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("radiant_gpu_signal_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
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
        Self {
            format,
            bind_group_layout,
            pipeline,
        }
    }
}

fn uniforms_as_bytes(uniforms: &GpuSurfaceUniforms) -> &[u8] {
    let len = std::mem::size_of::<GpuSurfaceUniforms>();
    let ptr = std::ptr::from_ref(uniforms).cast::<u8>();
    // SAFETY: `GpuSurfaceUniforms` is a plain repr(C) float-only uniform block.
    unsafe { std::slice::from_raw_parts(ptr, len) }
}

fn signal_uniforms_as_bytes(uniforms: &SignalUniforms) -> &[u8] {
    let len = std::mem::size_of::<SignalUniforms>();
    let ptr = std::ptr::from_ref(uniforms).cast::<u8>();
    // SAFETY: `SignalUniforms` is a plain repr(C) float-only uniform block.
    unsafe { std::slice::from_raw_parts(ptr, len) }
}

fn f32s_as_bytes(values: &[f32]) -> &[u8] {
    let len = std::mem::size_of_val(values);
    let ptr = values.as_ptr().cast::<u8>();
    // SAFETY: `f32` samples are plain data and the byte view does not outlive `values`.
    unsafe { std::slice::from_raw_parts(ptr, len) }
}

fn surface_dest(surface: &PaintGpuSurface) -> [f32; 4] {
    [
        surface.rect.min.x,
        surface.rect.min.y,
        surface.rect.width(),
        surface.rect.height(),
    ]
}

fn gpu_surface_render_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    target_view: &'a wgpu::TextureView,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("radiant_gpu_surface_pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target_view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    })
}

fn signal_body_render_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    target_view: &'a wgpu::TextureView,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("radiant_gpu_signal_body_pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target_view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    })
}

fn set_surface_scissor(pass: &mut wgpu::RenderPass<'_>, rect: UiRect) {
    let x = rect.min.x.max(0.0).floor() as u32;
    let y = rect.min.y.max(0.0).floor() as u32;
    let width = rect.width().max(1.0).ceil() as u32;
    let height = rect.height().max(1.0).ceil() as u32;
    pass.set_scissor_rect(x, y, width, height);
}

fn vertical_cursor(overlays: &[GpuSurfaceOverlay]) -> Option<(f32, Rgba8, f32)> {
    overlays.iter().find_map(|overlay| match *overlay {
        GpuSurfaceOverlay::VerticalCursor {
            ratio,
            color,
            width,
        } => Some((ratio, color, width)),
    })
}

fn rgba_to_float(color: Rgba8) -> [f32; 4] {
    [
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
        color.a as f32 / 255.0,
    ]
}

const GPU_SURFACE_SHADER: &str = r#"
struct Params {
    dest: vec4<f32>,
    source: vec4<f32>,
    target_size: vec2<f32>,
    cursor_ratio: f32,
    cursor_width: f32,
    cursor_color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> params: Params;
@group(0) @binding(1)
var surface_texture: texture_2d<f32>;
@group(0) @binding(2)
var surface_sampler: sampler;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
    );
    let local = corners[vertex_index];
    let pixel = params.dest.xy + local * params.dest.zw;
    let clip = vec2<f32>(
        pixel.x / params.target_size.x * 2.0 - 1.0,
        1.0 - pixel.y / params.target_size.y * 2.0,
    );
    let texture_size = vec2<f32>(textureDimensions(surface_texture));
    let source_pixel = params.source.xy + local * params.source.zw;
    var out: VertexOut;
    out.position = vec4<f32>(clip, 0.0, 1.0);
    out.local = local;
    out.uv = source_pixel / texture_size;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    var color = textureSample(surface_texture, surface_sampler, in.uv);
    let cursor_half_width = max(params.cursor_width / max(params.dest.z, 1.0), 0.0005);
    if (params.cursor_ratio >= 0.0 && abs(in.local.x - params.cursor_ratio) <= cursor_half_width) {
        color = params.cursor_color;
    }
    return color;
}
"#;

const GPU_SIGNAL_SHADER: &str = r#"
struct Params {
    dest: vec4<f32>,
    frame_range: vec4<f32>,
    target_size: vec2<f32>,
    cursor_ratio: f32,
    cursor_width: f32,
    cursor_color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> params: Params;
@group(0) @binding(1)
var<storage, read> samples: array<f32>;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
    );
    let local = corners[vertex_index];
    let pixel = params.dest.xy + local * params.dest.zw;
    let clip = vec2<f32>(
        pixel.x / params.target_size.x * 2.0 - 1.0,
        1.0 - pixel.y / params.target_size.y * 2.0,
    );
    var out: VertexOut;
    out.position = vec4<f32>(clip, 0.0, 1.0);
    out.local = local;
    return out;
}

fn sample_band(frame: u32, band: u32, frames: u32, band_count: u32) -> f32 {
    if (frame >= frames || band >= band_count) {
        return 0.0;
    }
    return samples[frame * band_count + band];
}

fn sample_band_linear(frame_pos: f32, band: u32, frames: u32, band_count: u32) -> f32 {
    let left_pos = clamp(floor(frame_pos), 0.0, f32(frames - 1u));
    let right_pos = clamp(left_pos + 1.0, 0.0, f32(frames - 1u));
    let t = clamp(frame_pos - left_pos, 0.0, 1.0);
    let left = sample_band(u32(left_pos), band, frames, band_count);
    let right = sample_band(u32(right_pos), band, frames, band_count);
    return mix(left, right, smoothstep(0.0, 1.0, t));
}

fn band_peak(start_frame: u32, end_frame: u32, band: u32, frames: u32, band_count: u32) -> f32 {
    var peak = 0.0;
    let span = max(end_frame - start_frame, 1u);
    let step = max(span / 40u, 1u);
    var frame = start_frame;
    loop {
        if (frame >= end_frame || frame >= frames) {
            break;
        }
        peak = max(peak, abs(sample_band(frame, band, frames, band_count)));
        frame = frame + step;
    }
    return peak;
}

fn band_peak_at(x: f32, pixel_width: f32, band: u32, frames: u32, band_count: u32, start: f32, visible: f32) -> f32 {
    let center = clamp(x, 0.0, 1.0);
    let a = u32(clamp(floor(start + visible * max(center - pixel_width * 0.5, 0.0)), 0.0, f32(frames - 1u)));
    let b = u32(clamp(ceil(start + visible * min(center + pixel_width * 0.5, 1.0)), 1.0, f32(frames)));
    return band_peak(a, max(b, a + 1u), band, frames, band_count);
}

fn smoothed_band_peak(x: f32, pixel_width: f32, band: u32, frames: u32, band_count: u32, start: f32, visible: f32) -> f32 {
    let p1 = band_peak_at(x - pixel_width, pixel_width, band, frames, band_count, start, visible);
    let p2 = band_peak_at(x, pixel_width, band, frames, band_count, start, visible);
    let p3 = band_peak_at(x + pixel_width, pixel_width, band, frames, band_count, start, visible);
    return p1 * 0.24 + p2 * 0.52 + p3 * 0.24;
}

fn corner_limited_band_peak(x: f32, pixel_width: f32, band: u32, frames: u32, band_count: u32, start: f32, visible: f32, frames_per_pixel: f32) -> f32 {
    let peak = smoothed_band_peak(x, pixel_width, band, frames, band_count, start, visible);
    let left = smoothed_band_peak(x - pixel_width, pixel_width, band, frames, band_count, start, visible);
    let right = smoothed_band_peak(x + pixel_width, pixel_width, band, frames, band_count, start, visible);
    let neighbor = max(left, right);
    let corner_limit = mix(0.24, 0.095, smoothstep(18.0, 260.0, frames_per_pixel));
    let corner_delta = max(peak - neighbor, 0.0);
    let corner_strength = smoothstep(corner_limit, corner_limit * 2.8, corner_delta);
    return mix(peak, neighbor + corner_limit, corner_strength * 0.82);
}

fn low_band_detail_envelope(x: f32, pixel_width: f32, band: u32, frames: u32, band_count: u32, start: f32, visible: f32) -> f32 {
    let frame_pos = clamp(start + visible * clamp(x, 0.0, 1.0), 0.0, f32(frames - 1u));
    let spread = max(visible * pixel_width, 8.0);
    let p0 = abs(sample_band_linear(frame_pos - spread * 2.0, band, frames, band_count));
    let p1 = abs(sample_band_linear(frame_pos - spread, band, frames, band_count));
    let p2 = abs(sample_band_linear(frame_pos, band, frames, band_count));
    let p3 = abs(sample_band_linear(frame_pos + spread, band, frames, band_count));
    let p4 = abs(sample_band_linear(frame_pos + spread * 2.0, band, frames, band_count));
    return p0 * 0.08 + p1 * 0.22 + p2 * 0.40 + p3 * 0.22 + p4 * 0.08;
}

fn low_band_closed_envelope(x: f32, pixel_width: f32, band: u32, frames: u32, band_count: u32, start: f32, visible: f32) -> f32 {
    let center = low_band_detail_envelope(x, pixel_width, band, frames, band_count, start, visible);
    let left = low_band_detail_envelope(x - pixel_width * 1.4, pixel_width, band, frames, band_count, start, visible);
    let right = low_band_detail_envelope(x + pixel_width * 1.4, pixel_width, band, frames, band_count, start, visible);
    let fill_narrow_dip = min(left, right) * 0.86;
    let close_strength = smoothstep(8.0, 72.0, visible * pixel_width);
    return mix(center, max(center, fill_narrow_dip), close_strength);
}

fn interpolated_band_extent(x: f32, pixel_width: f32, band: u32, frames: u32, band_count: u32, start: f32, visible: f32) -> f32 {
    let frame_pos = clamp(start + visible * clamp(x, 0.0, 1.0), 0.0, f32(frames - 1u));
    let center = abs(sample_band_linear(frame_pos, band, frames, band_count));
    let left = abs(sample_band_linear(frame_pos - visible * pixel_width, band, frames, band_count));
    let right = abs(sample_band_linear(frame_pos + visible * pixel_width, band, frames, band_count));
    return left * 0.18 + center * 0.64 + right * 0.18;
}

fn blend(src: vec3<f32>, alpha: f32, dst: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(mix(dst.rgb, src, clamp(alpha, 0.0, 1.0)), 1.0);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let frames = u32(max(params.frame_range.z, 1.0));
    let band_count = u32(max(params.frame_range.w, 1.0));
    let start = params.frame_range.x;
    let end = max(params.frame_range.y, start + 1.0);
    let visible = end - start;
    let pixel_width = 1.0 / max(params.dest.z, 1.0);
    let frames_per_pixel = visible * pixel_width;
    let y = abs(in.local.y - 0.5) * 2.0;
    let base_feather = max(0.55 / max(params.dest.y, 1.0), 0.00075);

    let vignette = (1.0 - y) * (1.0 - y);
    var color = vec4<f32>(0.004, 0.005, 0.005, 1.0);
    color = blend(vec3<f32>(0.012, 0.030, 0.044), vignette * 0.18, color);

    let band_colors = array<vec4<f32>, 4>(
        vec4<f32>(0.015, 0.16, 0.82, 0.46),
        vec4<f32>(0.58, 0.25, 0.07, 0.42),
        vec4<f32>(0.98, 0.46, 0.07, 0.50),
        vec4<f32>(1.00, 0.92, 0.66, 0.62),
    );
    let inner_colors = array<vec3<f32>, 4>(
        vec3<f32>(0.06, 0.70, 1.00),
        vec3<f32>(0.92, 0.44, 0.16),
        vec3<f32>(1.00, 0.68, 0.20),
        vec3<f32>(1.00, 1.00, 0.88),
    );
    let ridge_colors = array<vec3<f32>, 4>(
        vec3<f32>(0.02, 0.92, 1.00),
        vec3<f32>(0.95, 0.52, 0.20),
        vec3<f32>(1.00, 0.76, 0.28),
        vec3<f32>(1.00, 1.00, 0.96),
    );
    let glow_colors = array<vec3<f32>, 4>(
        vec3<f32>(0.00, 0.34, 1.00),
        vec3<f32>(0.78, 0.23, 0.04),
        vec3<f32>(1.00, 0.36, 0.02),
        vec3<f32>(1.00, 0.82, 0.36),
    );
    let band_scales = array<f32, 4>(1.00, 0.82, 0.72, 0.48);
    var low_signal = 0.0;
    var mid_signal = 0.0;
    var high_signal = 0.0;
    for (var band = 0u; band < min(band_count, 4u); band = band + 1u) {
        var peak = 0.0;
        if (frames_per_pixel <= 2.0) {
            peak = interpolated_band_extent(in.local.x, pixel_width, band, frames, band_count, start, visible);
        } else {
            peak = corner_limited_band_peak(in.local.x, pixel_width, band, frames, band_count, start, visible, frames_per_pixel);
        }
        if (band == 0u) {
            low_signal = peak;
        } else if (band == min(2u, band_count - 1u)) {
            mid_signal = peak;
        } else if (band == min(3u, band_count - 1u)) {
            high_signal = peak;
        }
        let intensity = clamp(peak * 1.04, 0.0, 1.0);
        let extent = peak * band_scales[band] * 0.86;
        let edge = abs(y - extent);
        let aa = max(fwidth(y - extent) * 0.90, base_feather);
        let coverage = smoothstep(extent + aa, extent - aa, y);
        let ridge = 1.0 - smoothstep(aa * 0.35, aa * 1.70, edge);
        let inside = clamp(1.0 - y / max(extent, 0.001), 0.0, 1.0);
        let inner_light = inside * inside;
        let shell_light = clamp(y / max(extent, 0.001), 0.0, 1.0);
        let edge_halo = smoothstep(extent + aa * 8.0, extent, y) * (1.0 - coverage);
        let heat_mix = smoothstep(0.38, 0.96, intensity);
        var low_depth = 0.0;
        var low_lift = vec3<f32>(0.0);
        var low_belly = 0.0;
        var ridge_seed = ridge_colors[band];
        if (band == 0u) {
            low_depth = smoothstep(0.03, 0.82, peak);
            let low_inner_cyan = low_depth * inside * inside * inside;
            let low_outer_blue = low_depth * smoothstep(0.28, 0.92, shell_light);
            let low_edge = low_depth * (1.0 - smoothstep(aa * 1.5, aa * 9.0, edge));
            let belly = clamp(1.0 - inside, 0.0, 1.0);
            low_belly = low_depth * belly * belly;
            low_lift = vec3<f32>(0.0, 0.17, 0.34) * low_outer_blue
                + vec3<f32>(0.0, 0.24, 0.44) * low_inner_cyan
                + vec3<f32>(0.0, 0.10, 0.20) * low_edge;
            ridge_seed = mix(ridge_seed, vec3<f32>(0.32, 0.95, 1.00), low_depth * 0.55);
        }
        let low_band = select(0.0, 1.0, band == 0u);
        let body_rgb = mix(
            mix(
                band_colors[band].rgb * (1.0 - low_belly * 0.18),
                inner_colors[band],
                inner_light * (0.46 + low_depth * 0.16),
            ) + low_lift,
            ridge_colors[band],
            shell_light * (0.14 + low_depth * 0.10) + heat_mix * 0.12,
        );
        let ridge_rgb = mix(
            ridge_seed,
            vec3<f32>(1.0, 0.86, 0.38),
            heat_mix * 0.30 * (1.0 - low_band * 0.70),
        );
        let alpha_boost = 0.62 + intensity * 0.28 + inner_light * 0.15;
        color = blend(glow_colors[band], edge_halo * band_colors[band].a * (0.06 + intensity * 0.09 + low_depth * 0.05), color);
        color = blend(body_rgb, band_colors[band].a * coverage * alpha_boost, color);
        color = blend(ridge_rgb, ridge * band_colors[band].a * (0.22 + intensity * 0.30), color);
    }
    let heat = clamp(low_signal * 0.45 + mid_signal * 0.70 + high_signal * 1.10, 0.0, 1.0);
    let hot = smoothstep(0.52, 0.98, heat);
    let center_width = max(5.0 / max(params.dest.y, 1.0), 0.014);
    let center = 1.0 - smoothstep(0.0, center_width, abs(in.local.y - 0.5));
    color = blend(vec3<f32>(1.00, 0.72, 0.28), center * (0.14 + hot * 0.14), color);
    color = blend(vec3<f32>(1.00, 0.98, 0.84), center * high_signal * 0.24, color);

    let cursor_half_width = max(params.cursor_width / max(params.dest.z, 1.0), 0.0005);
    if (params.cursor_ratio >= 0.0 && abs(in.local.x - params.cursor_ratio) <= cursor_half_width) {
        color = params.cursor_color;
    }
    return color;
}
"#;
