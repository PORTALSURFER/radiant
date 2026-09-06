use std::time::Duration;

use super::upload_plan::{
    GpuSurfaceRenderCanvasUploadClass, GpuSurfaceRenderCanvasUploadPlan,
    GpuSurfaceRenderCanvasUploadPlanContext, GpuSurfaceRenderCanvasUploadPlanObservation,
    GpuSurfaceRenderCanvasUploadPlanUnavailableReason,
};

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GpuSurfaceRenderStats {
    pub(crate) persistent_storage_complete: bool,
    pub(crate) atlas: GpuSurfaceAtlasRenderStats,
    pub(crate) signal: GpuSurfaceSignalRenderStats,
    pub(crate) composite: GpuSurfaceCompositeRenderStats,
    pub(crate) custom_shader: GpuSurfaceCustomShaderRenderStats,
    pub(crate) render_canvas_uploads: GpuSurfaceRenderCanvasUploadStats,
    pub(crate) render_canvas_upload_plan: Option<GpuSurfaceRenderCanvasUploadPlanObservation>,
}

impl GpuSurfaceRenderStats {
    pub(crate) fn with_upload_plan(
        context: Option<GpuSurfaceRenderCanvasUploadPlanContext>,
    ) -> Self {
        Self {
            render_canvas_upload_plan: context
                .map(|_| GpuSurfaceRenderCanvasUploadPlanObservation::NoWork),
            ..Self::default()
        }
    }

    pub(crate) fn record_candidate_immutable_payload(&mut self, byte_len: usize) {
        if let Some(observation) = self.render_canvas_upload_plan.as_mut() {
            GpuSurfaceRenderCanvasUploadPlan::record_observation(
                observation,
                GpuSurfaceRenderCanvasUploadClass::ImmutablePayload,
                byte_len,
            );
        }
    }

    pub(crate) fn record_candidate_volatile_payload(&mut self, byte_len: usize) {
        if let Some(observation) = self.render_canvas_upload_plan.as_mut() {
            GpuSurfaceRenderCanvasUploadPlan::record_observation(
                observation,
                GpuSurfaceRenderCanvasUploadClass::VolatilePayload,
                byte_len,
            );
        }
    }

    pub(crate) fn record_candidate_renderer_parameter(&mut self, byte_len: usize) {
        if let Some(observation) = self.render_canvas_upload_plan.as_mut() {
            GpuSurfaceRenderCanvasUploadPlan::record_observation(
                observation,
                GpuSurfaceRenderCanvasUploadClass::RendererParameter,
                byte_len,
            );
        }
    }

    pub(crate) fn mark_candidate_unavailable(
        &mut self,
        reason: GpuSurfaceRenderCanvasUploadPlanUnavailableReason,
    ) {
        if let Some(observation) = self.render_canvas_upload_plan.as_mut() {
            GpuSurfaceRenderCanvasUploadPlan::mark_observation_unavailable(observation, reason);
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GpuSurfaceAtlasRenderStats {
    pub(crate) texture_uploads: usize,
    pub(crate) texture_cache_hits: usize,
    pub(crate) texture_revision_mismatches: usize,
    pub(crate) texture_content_mismatches: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GpuSurfaceSignalRenderStats {
    pub(crate) summary_builds: usize,
    pub(crate) summary_cache_hits: usize,
    pub(crate) summary_revision_mismatches: usize,
    pub(crate) summary_content_mismatches: usize,
    pub(crate) summary_validation_runs: usize,
    pub(crate) summary_validation_cache_hits: usize,
    pub(crate) body_renders: usize,
    pub(crate) body_cache_hits: usize,
    pub(crate) body_revision_mismatches: usize,
    pub(crate) body_content_mismatches: usize,
    pub(crate) body_encode_elapsed: Duration,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GpuSurfaceCompositeRenderStats {
    pub(crate) binding_rebuilds: usize,
    pub(crate) binding_cache_hits: usize,
    pub(crate) binding_revision_mismatches: usize,
    pub(crate) binding_content_mismatches: usize,
    pub(crate) encode_elapsed: Duration,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GpuSurfaceCustomShaderRenderStats {
    pub(crate) surfaces_rendered: usize,
    pub(crate) pipeline_rebuilds: usize,
    pub(crate) binding_rebuilds: usize,
    pub(crate) binding_cache_hits: usize,
    pub(crate) static_writes: usize,
    pub(crate) static_write_bytes: usize,
    pub(crate) presentation_writes: usize,
    pub(crate) presentation_write_bytes: usize,
    pub(crate) failures: GpuSurfaceCustomShaderFailureStats,
    pub(crate) unsupported: GpuSurfaceUnsupportedCustomShaderStats,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GpuSurfaceCustomShaderFailureStats {
    pub(crate) surfaces_failed: usize,
    pub(crate) shader_module_failures: usize,
    pub(crate) pipeline_failures: usize,
    pub(crate) binding_failures: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GpuSurfaceUnsupportedCustomShaderStats {
    pub(crate) surfaces: usize,
    pub(crate) vertices: usize,
    pub(crate) source_bytes: usize,
    pub(crate) uniform_bytes: usize,
    pub(crate) storage_bytes: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct GpuSurfaceRenderCanvasUploadStats {
    pub(crate) immutable_payload: GpuSurfaceRenderCanvasUploadEvidence,
    pub(crate) volatile_payload: GpuSurfaceRenderCanvasUploadEvidence,
    pub(crate) renderer_parameter: GpuSurfaceRenderCanvasUploadEvidence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GpuSurfaceRenderCanvasUploadEvidence {
    pub(crate) operations: Option<usize>,
    pub(crate) logical_bytes: Option<u64>,
}

impl Default for GpuSurfaceRenderCanvasUploadEvidence {
    fn default() -> Self {
        Self {
            operations: Some(0),
            logical_bytes: Some(0),
        }
    }
}

impl GpuSurfaceRenderCanvasUploadStats {
    pub(crate) fn record_immutable_payload(&mut self, byte_len: usize) {
        self.immutable_payload.record(byte_len);
    }

    pub(crate) fn record_volatile_payload(&mut self, byte_len: usize) {
        self.volatile_payload.record(byte_len);
    }

    pub(crate) fn record_renderer_parameter(&mut self, byte_len: usize) {
        self.renderer_parameter.record(byte_len);
    }
}

impl GpuSurfaceRenderCanvasUploadEvidence {
    fn record(&mut self, byte_len: usize) {
        let (Some(operations), Some(logical_bytes)) = (self.operations, self.logical_bytes) else {
            return;
        };
        let Some(byte_len) = u64::try_from(byte_len).ok() else {
            self.mark_unavailable();
            return;
        };
        let (Some(operations), Some(logical_bytes)) = (
            operations.checked_add(1),
            logical_bytes.checked_add(byte_len),
        ) else {
            self.mark_unavailable();
            return;
        };
        self.operations = Some(operations);
        self.logical_bytes = Some(logical_bytes);
    }

    fn mark_unavailable(&mut self) {
        self.operations = None;
        self.logical_bytes = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_stats_track_composite_binding_cache_activity() {
        let stats = GpuSurfaceRenderStats::default();

        assert_eq!(stats.composite.binding_rebuilds, 0);
        assert_eq!(stats.composite.binding_cache_hits, 0);
        assert_eq!(stats.custom_shader.surfaces_rendered, 0);
        assert_eq!(stats.custom_shader.pipeline_rebuilds, 0);
        assert_eq!(stats.custom_shader.binding_rebuilds, 0);
        assert_eq!(stats.custom_shader.binding_cache_hits, 0);
        assert_eq!(stats.custom_shader.static_writes, 0);
        assert_eq!(stats.custom_shader.static_write_bytes, 0);
        assert_eq!(stats.custom_shader.presentation_writes, 0);
        assert_eq!(stats.custom_shader.presentation_write_bytes, 0);
        assert_eq!(stats.custom_shader.failures.surfaces_failed, 0);
        assert_eq!(stats.custom_shader.failures.shader_module_failures, 0);
        assert_eq!(stats.custom_shader.failures.pipeline_failures, 0);
        assert_eq!(stats.custom_shader.failures.binding_failures, 0);
        assert_eq!(stats.custom_shader.unsupported.surfaces, 0);
        assert_eq!(stats.custom_shader.unsupported.vertices, 0);
        assert_eq!(stats.custom_shader.unsupported.source_bytes, 0);
        assert_eq!(stats.custom_shader.unsupported.uniform_bytes, 0);
        assert_eq!(stats.custom_shader.unsupported.storage_bytes, 0);
        assert_eq!(
            stats.render_canvas_uploads,
            GpuSurfaceRenderCanvasUploadStats::default()
        );
    }

    #[test]
    fn render_stats_track_atlas_texture_cache_activity() {
        let stats = GpuSurfaceRenderStats::default();

        assert_eq!(stats.atlas.texture_uploads, 0);
        assert_eq!(stats.atlas.texture_cache_hits, 0);
    }

    #[test]
    fn disabled_upload_plan_collection_keeps_candidate_absent() {
        let stats = GpuSurfaceRenderStats::with_upload_plan(None);

        assert!(stats.render_canvas_upload_plan.is_none());
    }

    #[test]
    fn render_canvas_upload_evidence_records_classified_operations_deterministically() {
        let mut first = GpuSurfaceRenderCanvasUploadStats::default();
        first.record_immutable_payload(16);
        first.record_immutable_payload(8);
        first.record_volatile_payload(12);
        first.record_renderer_parameter(240);
        first.record_renderer_parameter(144);

        let mut second = GpuSurfaceRenderCanvasUploadStats::default();
        second.record_immutable_payload(16);
        second.record_immutable_payload(8);
        second.record_volatile_payload(12);
        second.record_renderer_parameter(240);
        second.record_renderer_parameter(144);

        assert_eq!(first, second);
        assert_eq!(
            first.immutable_payload,
            GpuSurfaceRenderCanvasUploadEvidence {
                operations: Some(2),
                logical_bytes: Some(24),
            }
        );
        assert_eq!(
            first.volatile_payload,
            GpuSurfaceRenderCanvasUploadEvidence {
                operations: Some(1),
                logical_bytes: Some(12),
            }
        );
        assert_eq!(
            first.renderer_parameter,
            GpuSurfaceRenderCanvasUploadEvidence {
                operations: Some(2),
                logical_bytes: Some(384),
            }
        );
    }

    #[test]
    fn render_canvas_upload_evidence_overflow_is_sticky_unavailable() {
        let mut operation_overflow = GpuSurfaceRenderCanvasUploadEvidence {
            operations: Some(usize::MAX),
            logical_bytes: Some(4),
        };
        operation_overflow.record(8);
        assert_eq!(operation_overflow.operations, None);
        assert_eq!(operation_overflow.logical_bytes, None);
        operation_overflow.record(8);
        assert_eq!(operation_overflow.operations, None);
        assert_eq!(operation_overflow.logical_bytes, None);

        let mut byte_overflow = GpuSurfaceRenderCanvasUploadEvidence {
            operations: Some(1),
            logical_bytes: Some(u64::MAX),
        };
        byte_overflow.record(1);
        assert_eq!(byte_overflow.operations, None);
        assert_eq!(byte_overflow.logical_bytes, None);
    }
}
