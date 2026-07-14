/// CPU work performed while preprocessing and querying one GPU-surface occlusion plan.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GpuSurfaceOcclusionPlanningDiagnostics {
    /// Paint primitives visited while preprocessing the plan.
    pub paint_primitives_visited: usize,
    /// Clip-resolved opaque rectangles added to the spatial suffix index.
    pub occluder_rects_indexed: usize,
    /// GPU surfaces considered by visibility planning.
    pub gpu_surfaces_planned: usize,
    /// GPU surfaces with at least one visible region.
    pub visible_gpu_surfaces: usize,
    /// Spatial index nodes visited across all surface queries.
    pub index_nodes_visited: usize,
    /// Leaf occluder candidates tested across all surface queries.
    pub occluder_candidates_visited: usize,
    /// Visible rectangles produced after clip and opaque-suffix subtraction.
    pub visible_regions_produced: usize,
}

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
