use super::*;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GpuSurfaceRenderStats {
    pub(crate) atlas: GpuSurfaceAtlasRenderStats,
    pub(crate) signal: GpuSurfaceSignalRenderStats,
    pub(crate) composite: GpuSurfaceCompositeRenderStats,
    pub(crate) custom_shader: GpuSurfaceCustomShaderRenderStats,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GpuSurfaceAtlasRenderStats {
    pub(crate) texture_uploads: usize,
    pub(crate) texture_cache_hits: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GpuSurfaceSignalRenderStats {
    pub(crate) summary_builds: usize,
    pub(crate) summary_cache_hits: usize,
    pub(crate) body_renders: usize,
    pub(crate) body_cache_hits: usize,
    pub(crate) body_encode_elapsed: Duration,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GpuSurfaceCompositeRenderStats {
    pub(crate) binding_rebuilds: usize,
    pub(crate) binding_cache_hits: usize,
    pub(crate) encode_elapsed: Duration,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GpuSurfaceCustomShaderRenderStats {
    pub(crate) surfaces_rendered: usize,
    pub(crate) pipeline_rebuilds: usize,
    pub(crate) binding_rebuilds: usize,
    pub(crate) binding_cache_hits: usize,
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
        assert_eq!(stats.custom_shader.failures.surfaces_failed, 0);
        assert_eq!(stats.custom_shader.failures.shader_module_failures, 0);
        assert_eq!(stats.custom_shader.failures.pipeline_failures, 0);
        assert_eq!(stats.custom_shader.failures.binding_failures, 0);
        assert_eq!(stats.custom_shader.unsupported.surfaces, 0);
        assert_eq!(stats.custom_shader.unsupported.vertices, 0);
        assert_eq!(stats.custom_shader.unsupported.source_bytes, 0);
        assert_eq!(stats.custom_shader.unsupported.uniform_bytes, 0);
        assert_eq!(stats.custom_shader.unsupported.storage_bytes, 0);
    }

    #[test]
    fn render_stats_track_atlas_texture_cache_activity() {
        let stats = GpuSurfaceRenderStats::default();

        assert_eq!(stats.atlas.texture_uploads, 0);
        assert_eq!(stats.atlas.texture_cache_hits, 0);
    }
}
