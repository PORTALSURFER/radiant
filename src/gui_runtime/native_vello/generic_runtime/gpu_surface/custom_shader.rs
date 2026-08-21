use super::stats::GpuSurfaceRenderStats;
use super::{GpuSurfaceRenderTarget, GpuSurfaceRenderer, PRESENTATION_STAGING_BELT_CHUNK_SIZE};
use crate::gui::types::Rect as UiRect;
use crate::runtime::{
    GpuShaderPresentationUniformUpdate, GpuShaderSurfaceDescriptor, GpuSurfaceContent,
    PaintGpuSurface,
};
#[path = "custom_shader/binding.rs"]
mod binding;
#[path = "custom_shader/diagnostics.rs"]
mod diagnostics;
#[path = "custom_shader/draw.rs"]
mod draw;
#[path = "custom_shader/pipeline.rs"]
mod pipeline;
use binding::CustomShaderBindingRequest;
use diagnostics::{record_failed_custom_shader_surface, record_unsupported_custom_shader};
use draw::{CustomShaderBufferUploadRequest, CustomShaderDrawRequest};
use pipeline::{CustomShaderPipelineRequest, custom_shader_pipeline_key};

impl GpuSurfaceRenderer {
    pub(super) fn render_custom_shader(
        &mut self,
        target: &mut GpuSurfaceRenderTarget<'_>,
        surface: &PaintGpuSurface,
        occlusion_regions: &[UiRect],
        presentation_updates: &[GpuShaderPresentationUniformUpdate],
        stats: &mut GpuSurfaceRenderStats,
    ) {
        let Some(descriptor) = supported_custom_shader_descriptor(surface, stats) else {
            return;
        };
        if !self.prepare_custom_shader_resources(target, surface, descriptor, stats) {
            return;
        }
        let presentation_update =
            matching_presentation_update(surface, descriptor, presentation_updates);
        let presentation_staging_belt = descriptor
            .presentation_uniform_bytes
            .as_ref()
            .filter(|bytes| !bytes.is_empty())
            .map(|_| {
                self.presentation_staging_belt.get_or_insert_with(|| {
                    vello::wgpu::util::StagingBelt::new(
                        target.device.clone(),
                        PRESENTATION_STAGING_BELT_CHUNK_SIZE,
                    )
                })
            });
        {
            let Some(binding) = self.resources.custom_shader_bindings.get_mut(&surface.key) else {
                record_failed_custom_shader_surface(stats);
                return;
            };
            draw::upload_custom_shader_buffers(
                CustomShaderBufferUploadRequest {
                    target,
                    surface,
                    descriptor,
                    binding,
                    presentation_update,
                    presentation_staging_belt,
                },
                stats,
            );
        }
        let Some(pipeline) = self.resources.custom_shader_pipelines.get(&surface.key) else {
            record_failed_custom_shader_surface(stats);
            return;
        };
        let Some(binding) = self.resources.custom_shader_bindings.get(&surface.key) else {
            record_failed_custom_shader_surface(stats);
            return;
        };
        draw::encode_custom_shader_draw(
            CustomShaderDrawRequest {
                target,
                surface,
                descriptor,
                pipeline,
                binding,
                occlusion_regions,
            },
            stats,
        );
        stats.custom_shader.surfaces_rendered += 1;
    }

    fn prepare_custom_shader_resources(
        &mut self,
        target: &GpuSurfaceRenderTarget<'_>,
        surface: &PaintGpuSurface,
        descriptor: &GpuShaderSurfaceDescriptor,
        stats: &mut GpuSurfaceRenderStats,
    ) -> bool {
        let Some(pipeline_key) = custom_shader_pipeline_key(descriptor) else {
            record_unsupported_custom_shader(descriptor, stats);
            return false;
        };
        self.ensure_custom_shader_pipeline(
            CustomShaderPipelineRequest {
                surface_key: surface.key,
                device: target.device,
                target_format: target.format,
                key: pipeline_key,
            },
            stats,
        );
        if !self
            .resources
            .custom_shader_pipelines
            .contains_key(&surface.key)
        {
            record_failed_custom_shader_surface(stats);
            return false;
        }
        self.ensure_custom_shader_binding(
            CustomShaderBindingRequest {
                device: target.device,
                surface_key: surface.key,
                descriptor,
            },
            stats,
        );
        if self
            .resources
            .custom_shader_bindings
            .contains_key(&surface.key)
        {
            true
        } else {
            record_failed_custom_shader_surface(stats);
            false
        }
    }
}

fn supported_custom_shader_descriptor<'a>(
    surface: &'a PaintGpuSurface,
    stats: &mut GpuSurfaceRenderStats,
) -> Option<&'a GpuShaderSurfaceDescriptor> {
    let GpuSurfaceContent::CustomShader { descriptor } = &surface.content else {
        return None;
    };
    if !custom_shader_descriptor_is_supported(descriptor) {
        record_unsupported_custom_shader(descriptor, stats);
        return None;
    }
    Some(descriptor)
}

pub(super) fn custom_shader_descriptor_is_supported(
    descriptor: &GpuShaderSurfaceDescriptor,
) -> bool {
    descriptor.wgsl_source.is_some() && descriptor.fragment_entry_point.is_some()
}

fn matching_presentation_update<'a>(
    surface: &PaintGpuSurface,
    descriptor: &GpuShaderSurfaceDescriptor,
    updates: &'a [GpuShaderPresentationUniformUpdate],
) -> Option<&'a GpuShaderPresentationUniformUpdate> {
    let presentation_byte_len = descriptor
        .presentation_uniform_bytes
        .as_ref()
        .map_or(0, |bytes| bytes.len());
    if presentation_byte_len == 0 {
        return None;
    }
    updates
        .iter()
        .filter(|update| {
            update.widget_id == surface.widget_id
                && update.surface_key.get() == surface.key
                && update.storage_identity == descriptor.storage_identity
                && update.storage_revision == descriptor.storage_revision
                && update.byte_len() == presentation_byte_len
        })
        .max_by_key(|update| update.presentation_revision)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        layout::{Point, Rect, Vector2},
        runtime::GpuSurfaceCapabilities,
    };
    use std::sync::Arc;

    #[test]
    fn descriptor_selection_ignores_non_custom_shader_surfaces() {
        let mut stats = GpuSurfaceRenderStats::default();
        let surface = PaintGpuSurface {
            widget_id: 17,
            key: 93,
            revision: 2,
            rect: test_rect(),
            content: GpuSurfaceContent::SignalBands {
                frames: 0,
                band_count: 1,
                frame_range: [0.0, 0.0],
                samples: Arc::from([]),
            },
            capabilities: GpuSurfaceCapabilities::default(),
            overlays: Vec::new(),
        };

        assert!(supported_custom_shader_descriptor(&surface, &mut stats).is_none());
        assert_eq!(stats.custom_shader.unsupported.surfaces, 0);
    }

    #[test]
    fn descriptor_selection_records_unsupported_custom_shader_payloads() {
        let mut stats = GpuSurfaceRenderStats::default();
        let surface = PaintGpuSurface {
            widget_id: 17,
            key: 93,
            revision: 2,
            rect: test_rect(),
            content: GpuSurfaceContent::CustomShader {
                descriptor: Arc::new(
                    GpuShaderSurfaceDescriptor::new("test/custom-shader").uniform_bytes([1, 2, 3]),
                ),
            },
            capabilities: GpuSurfaceCapabilities::default(),
            overlays: Vec::new(),
        };

        assert!(supported_custom_shader_descriptor(&surface, &mut stats).is_none());
        assert_eq!(stats.custom_shader.unsupported.surfaces, 1);
        assert_eq!(stats.custom_shader.unsupported.uniform_bytes, 3);
    }

    #[test]
    fn presentation_updates_match_surface_storage_and_byte_shape() {
        let descriptor = GpuShaderSurfaceDescriptor::new("test/custom-shader")
            .storage_identity(11)
            .storage_revision(13)
            .presentation_uniform([1, 2, 3, 4], 2);
        let surface = PaintGpuSurface {
            widget_id: 17,
            key: 93,
            revision: 2,
            rect: test_rect(),
            content: GpuSurfaceContent::CustomShader {
                descriptor: Arc::new(descriptor.clone()),
            },
            capabilities: GpuSurfaceCapabilities::default(),
            overlays: Vec::new(),
        };
        let matching = GpuShaderPresentationUniformUpdate::try_new(17, 93, 11, 13, 4, [4, 5, 6, 7])
            .expect("valid matching presentation update");
        let wrong_storage =
            GpuShaderPresentationUniformUpdate::try_new(17, 93, 12, 13, 5, [7, 8, 9, 10])
                .expect("valid mismatched presentation update");
        let wrong_length = GpuShaderPresentationUniformUpdate::try_new(17, 93, 11, 13, 6, [10; 8])
            .expect("valid mismatched presentation update");

        assert_eq!(
            matching_presentation_update(&surface, &descriptor, &[wrong_storage, matching]),
            Some(&matching)
        );
        assert!(matching_presentation_update(&surface, &descriptor, &[wrong_length]).is_none());
    }

    fn test_rect() -> Rect {
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 24.0))
    }
}
