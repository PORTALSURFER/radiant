use super::*;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GpuSurfaceRenderStats {
    pub(crate) atlas_texture_uploads: usize,
    pub(crate) atlas_texture_cache_hits: usize,
    pub(crate) signal_summary_builds: usize,
    pub(crate) signal_summary_cache_hits: usize,
    pub(crate) signal_body_renders: usize,
    pub(crate) signal_body_cache_hits: usize,
    pub(crate) composite_binding_rebuilds: usize,
    pub(crate) composite_binding_cache_hits: usize,
    pub(crate) custom_shader_surfaces_rendered: usize,
    pub(crate) custom_shader_pipeline_rebuilds: usize,
    pub(crate) custom_shader_binding_rebuilds: usize,
    pub(crate) custom_shader_binding_cache_hits: usize,
    pub(crate) custom_shader_surfaces_failed: usize,
    pub(crate) custom_shader_shader_module_failures: usize,
    pub(crate) custom_shader_pipeline_failures: usize,
    pub(crate) custom_shader_binding_failures: usize,
    pub(crate) unsupported_custom_shader_surfaces: usize,
    pub(crate) unsupported_custom_shader_vertices: usize,
    pub(crate) unsupported_custom_shader_source_bytes: usize,
    pub(crate) unsupported_custom_shader_uniform_bytes: usize,
    pub(crate) unsupported_custom_shader_storage_bytes: usize,
    pub(crate) signal_body_encode_elapsed: Duration,
    pub(crate) composite_encode_elapsed: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_stats_track_composite_binding_cache_activity() {
        let stats = GpuSurfaceRenderStats::default();

        assert_eq!(stats.composite_binding_rebuilds, 0);
        assert_eq!(stats.composite_binding_cache_hits, 0);
        assert_eq!(stats.custom_shader_surfaces_rendered, 0);
        assert_eq!(stats.custom_shader_pipeline_rebuilds, 0);
        assert_eq!(stats.custom_shader_binding_rebuilds, 0);
        assert_eq!(stats.custom_shader_binding_cache_hits, 0);
        assert_eq!(stats.custom_shader_surfaces_failed, 0);
        assert_eq!(stats.custom_shader_shader_module_failures, 0);
        assert_eq!(stats.custom_shader_pipeline_failures, 0);
        assert_eq!(stats.custom_shader_binding_failures, 0);
        assert_eq!(stats.unsupported_custom_shader_surfaces, 0);
        assert_eq!(stats.unsupported_custom_shader_vertices, 0);
        assert_eq!(stats.unsupported_custom_shader_source_bytes, 0);
        assert_eq!(stats.unsupported_custom_shader_uniform_bytes, 0);
        assert_eq!(stats.unsupported_custom_shader_storage_bytes, 0);
    }

    #[test]
    fn render_stats_track_atlas_texture_cache_activity() {
        let stats = GpuSurfaceRenderStats::default();

        assert_eq!(stats.atlas_texture_uploads, 0);
        assert_eq!(stats.atlas_texture_cache_hits, 0);
    }
}
