/// GPU-surface cache and render diagnostics for one native frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeGpuSurfaceDiagnostics {
    /// Atlas texture uploads performed this frame.
    pub atlas_texture_uploads: usize,
    /// Atlas texture cache hits this frame.
    pub atlas_texture_cache_hits: usize,
    /// Signal summary buffers built this frame.
    pub signal_summary_builds: usize,
    /// Signal summary cache hits this frame.
    pub signal_summary_cache_hits: usize,
    /// Signal body renders encoded this frame.
    pub signal_body_renders: usize,
    /// Signal body cache hits this frame.
    pub signal_body_cache_hits: usize,
    /// Composite binding groups rebuilt this frame.
    pub composite_binding_rebuilds: usize,
    /// Composite binding cache hits this frame.
    pub composite_binding_cache_hits: usize,
    /// Custom-shader GPU surfaces encoded by the native WGPU path this frame.
    pub custom_shader_surfaces_rendered: usize,
    /// Custom-shader render pipelines rebuilt this frame.
    pub custom_shader_pipeline_rebuilds: usize,
    /// Custom-shader bind groups rebuilt this frame.
    pub custom_shader_binding_rebuilds: usize,
    /// Custom-shader bind groups reused from cache this frame.
    pub custom_shader_binding_cache_hits: usize,
    /// Custom-shader GPU surfaces that could not be encoded after native setup failed.
    pub custom_shader_surfaces_failed: usize,
    /// Custom-shader WGSL module validation failures observed this frame.
    pub custom_shader_shader_module_failures: usize,
    /// Custom-shader render-pipeline validation failures observed this frame.
    pub custom_shader_pipeline_failures: usize,
    /// Custom-shader bind-group validation failures observed this frame.
    pub custom_shader_binding_failures: usize,
    /// Valid custom-shader GPU surfaces skipped by this native backend.
    pub unsupported_custom_shader_surfaces: usize,
    /// Total vertex count requested by skipped custom-shader GPU surfaces.
    pub unsupported_custom_shader_vertices: usize,
    /// Total WGSL source bytes carried by skipped custom-shader GPU surfaces.
    pub unsupported_custom_shader_source_bytes: usize,
    /// Total uniform payload bytes carried by skipped custom-shader GPU surfaces.
    pub unsupported_custom_shader_uniform_bytes: usize,
    /// Total storage payload bytes carried by skipped custom-shader GPU surfaces.
    pub unsupported_custom_shader_storage_bytes: usize,
}
