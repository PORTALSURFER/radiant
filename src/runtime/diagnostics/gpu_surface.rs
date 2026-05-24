/// GPU-surface cache and render diagnostics for one native frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeGpuSurfaceDiagnostics {
    /// Retained atlas texture upload and cache activity.
    pub atlas: NativeGpuSurfaceAtlasDiagnostics,
    /// Signal summary and body cache activity.
    pub signal: NativeGpuSurfaceSignalDiagnostics,
    /// Composite binding-group cache activity.
    pub composite: NativeGpuSurfaceCompositeDiagnostics,
    /// Native WGPU custom-shader render activity.
    pub custom_shader: NativeGpuSurfaceCustomShaderDiagnostics,
}

/// Retained atlas texture diagnostics for one native frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeGpuSurfaceAtlasDiagnostics {
    /// Atlas texture uploads performed this frame.
    pub texture_uploads: usize,
    /// Atlas texture cache hits this frame.
    pub texture_cache_hits: usize,
}

/// Signal GPU-surface diagnostics for one native frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeGpuSurfaceSignalDiagnostics {
    /// Signal summary buffers built this frame.
    pub summary_builds: usize,
    /// Signal summary cache hits this frame.
    pub summary_cache_hits: usize,
    /// Signal body renders encoded this frame.
    pub body_renders: usize,
    /// Signal body cache hits this frame.
    pub body_cache_hits: usize,
}

/// Composite binding diagnostics for one native frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeGpuSurfaceCompositeDiagnostics {
    /// Composite binding groups rebuilt this frame.
    pub binding_rebuilds: usize,
    /// Composite binding groups reused from cache this frame.
    pub binding_cache_hits: usize,
}

/// Native WGPU custom-shader diagnostics for one native frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeGpuSurfaceCustomShaderDiagnostics {
    /// Custom-shader GPU surfaces encoded by the native WGPU path this frame.
    pub surfaces_rendered: usize,
    /// Custom-shader render pipelines rebuilt this frame.
    pub pipeline_rebuilds: usize,
    /// Custom-shader bind groups rebuilt this frame.
    pub binding_rebuilds: usize,
    /// Custom-shader bind groups reused from cache this frame.
    pub binding_cache_hits: usize,
    /// Custom-shader native setup failures observed this frame.
    pub failures: NativeGpuSurfaceCustomShaderFailureDiagnostics,
    /// Valid custom-shader surfaces skipped by this native backend.
    pub unsupported: NativeGpuSurfaceUnsupportedCustomShaderDiagnostics,
}

/// Native WGPU custom-shader failure diagnostics for one native frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeGpuSurfaceCustomShaderFailureDiagnostics {
    /// Custom-shader GPU surfaces that could not be encoded after native setup failed.
    pub surfaces_failed: usize,
    /// Custom-shader WGSL module validation failures observed this frame.
    pub shader_module_failures: usize,
    /// Custom-shader render-pipeline validation failures observed this frame.
    pub pipeline_failures: usize,
    /// Custom-shader bind-group validation failures observed this frame.
    pub binding_failures: usize,
}

/// Custom-shader surfaces skipped by this native backend for one native frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeGpuSurfaceUnsupportedCustomShaderDiagnostics {
    /// Valid custom-shader GPU surfaces skipped by this native backend.
    pub surfaces: usize,
    /// Total vertex count requested by skipped custom-shader GPU surfaces.
    pub vertices: usize,
    /// Total WGSL source bytes carried by skipped custom-shader GPU surfaces.
    pub source_bytes: usize,
    /// Total uniform payload bytes carried by skipped custom-shader GPU surfaces.
    pub uniform_bytes: usize,
    /// Total storage payload bytes carried by skipped custom-shader GPU surfaces.
    pub storage_bytes: usize,
}
