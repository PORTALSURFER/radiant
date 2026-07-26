use super::super::encoding::{
    signal_uniforms_as_bytes, summary_bucket_bytes, summary_bucket_value_count,
};
use super::super::gpu_surface_types::{
    SignalBodyCacheKey, SignalBodyTexture, SignalBuffer, SignalBufferCacheKey, SignalUniforms,
};
use super::super::identity::RenderCanvasContentOwner;
use super::super::passes::signal_body_render_pass;
use super::super::stats::GpuSurfaceRenderStats;
use super::super::{GpuSurfaceRenderer, wgpu_device_id};
use crate::runtime::GpuSignalSummaryBucket;
use std::time::Instant;
use vello::wgpu;
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
            stats.signal.body_cache_hits += 1;
            return Some(body.view.clone());
        }
        if let Some(body) = self.resources.signal_bodies.get(&key)
            && body.device == wgpu_device_id(device)
        {
            match signal_body_cache_mismatch(body.cache_key, body_key) {
                Some(SignalBodyCacheMismatch::Revision) => {
                    stats.signal.body_revision_mismatches += 1;
                }
                Some(SignalBodyCacheMismatch::Content) => {
                    stats.signal.body_content_mismatches += 1;
                }
                None => {}
            }
        }
        let buffer = self.resources.signals.get(&key)?;
        let content_owner = buffer._content_owner.clone();
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
        stats.signal.body_renders += 1;
        stats.signal.body_encode_elapsed += started.elapsed();
        let cached_view = view.clone();
        self.resources.signal_bodies.insert(
            key,
            SignalBodyTexture {
                device: wgpu_device_id(device),
                cache_key: body_key,
                _content_owner: content_owner,
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
        content_owner: RenderCanvasContentOwner,
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
                _content_owner: content_owner,
                _sample_buffer: sample_buffer,
                uniform_buffer,
                bind_group,
            },
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SignalBodyCacheMismatch {
    Revision,
    Content,
}

fn signal_body_cache_mismatch(
    cached: SignalBodyCacheKey,
    target: SignalBodyCacheKey,
) -> Option<SignalBodyCacheMismatch> {
    if cached.revision != target.revision {
        Some(SignalBodyCacheMismatch::Revision)
    } else if cached.content_identity != target.content_identity {
        Some(SignalBodyCacheMismatch::Content)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_runtime::native_vello::generic_runtime::gpu_surface::gpu_surface_types::SignalBodyCacheKeyParts;
    use crate::gui_runtime::native_vello::generic_runtime::gpu_surface::identity::RenderCanvasContentIdentity;

    fn body_key(
        revision: u64,
        content_identity: RenderCanvasContentIdentity,
    ) -> SignalBodyCacheKey {
        SignalBodyCacheKey::new(SignalBodyCacheKeyParts {
            revision,
            content_identity,
            extent: super::super::super::passes::SurfacePixelExtent {
                width: 64,
                height: 32,
            },
            frames: 128,
            band_count: 2,
            frame_range: [0.0, 1.0],
            sample_slide_frame_offset: 0,
            sample_count: 256,
            level_index: 0,
            gain_preview: None,
        })
    }

    #[test]
    fn signal_body_cache_mismatch_reports_revision_or_content() {
        let identity = RenderCanvasContentIdentity::default();
        let replacement = RenderCanvasContentIdentity::SignalBands {
            samples: 1,
            frames: 128,
            band_count: 2,
            frame_range: [0.0f32.to_bits(), 1.0f32.to_bits()],
        };
        assert_eq!(
            signal_body_cache_mismatch(body_key(1, identity), body_key(2, identity)),
            Some(SignalBodyCacheMismatch::Revision)
        );
        assert_eq!(
            signal_body_cache_mismatch(body_key(1, identity), body_key(1, replacement)),
            Some(SignalBodyCacheMismatch::Content)
        );
        assert_eq!(
            signal_body_cache_mismatch(body_key(1, identity), body_key(1, identity)),
            None
        );
    }
}
