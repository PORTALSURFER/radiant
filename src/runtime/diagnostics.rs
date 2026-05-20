use std::time::Duration;

/// Retained-surface frame cache policy for native renderers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetainedSurfaceCachePolicy {
    /// Maximum retained custom-surface frames held by the runtime.
    ///
    /// A value of zero disables retained-frame reuse while preserving normal
    /// retained-surface rendering.
    pub max_frames: usize,
}

impl RetainedSurfaceCachePolicy {
    /// Build a retained-surface cache policy with an explicit frame capacity.
    pub const fn max_frames(max_frames: usize) -> Self {
        Self { max_frames }
    }
}

impl Default for RetainedSurfaceCachePolicy {
    fn default() -> Self {
        Self { max_frames: 64 }
    }
}

/// Structured diagnostics for one native presentation frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeFrameDiagnostics {
    /// Scene and retained-surface encoding counters.
    pub scene: NativeSceneDiagnostics,
    /// Native text layout cache activity.
    pub text: NativeTextDiagnostics,
    /// Retained custom-surface cache state and activity.
    pub retained_surfaces: NativeRetainedSurfaceDiagnostics,
    /// GPU-surface cache and render activity.
    pub gpu_surfaces: NativeGpuSurfaceDiagnostics,
    /// Coarse timing buckets for presentation work.
    pub timings: NativeFrameTimingDiagnostics,
}

/// Native text layout cache diagnostics for one native frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeTextDiagnostics {
    /// Text-layout cache hits observed while preparing this frame.
    pub layout_cache_hits: u64,
    /// Text-layout cache misses observed while preparing this frame.
    pub layout_cache_misses: u64,
    /// Text-layout cache evictions observed while preparing this frame.
    pub layout_cache_evictions: u64,
    /// Text atom cache hits observed while preparing this frame.
    pub atom_cache_hits: u64,
    /// Text atom cache misses observed while preparing this frame.
    pub atom_cache_misses: u64,
    /// Text atom cache evictions observed while preparing this frame.
    pub atom_cache_evictions: u64,
    /// Glyphs substituted with the renderer's fallback glyph this frame.
    pub fallback_glyphs: u64,
    /// Glyphs the active native font could not resolve even through fallback.
    pub missing_glyphs: u64,
}

/// Scene encoding counters for one native frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeSceneDiagnostics {
    /// Paint-plan primitives visited while encoding the scene.
    pub paint_plan_primitives: usize,
    /// Clip layers pushed into the scene.
    pub clip_layer_count: usize,
    /// Text primitives visited before batching into scene text runs.
    pub text_primitive_count: usize,
    /// Text input primitives encoded.
    pub text_input_count: usize,
    /// Image primitives encoded.
    pub image_count: usize,
    /// SVG documents encoded.
    pub svg_document_count: usize,
    /// GPU-surface primitives visited.
    pub gpu_surface_count: usize,
    /// Retained/custom surface primitives visited.
    pub custom_surface_count: usize,
    /// Custom surfaces that fell back to placeholder rendering.
    pub custom_surface_fallback_count: u32,
    /// Total text runs submitted to the scene.
    pub text_run_count: usize,
}

/// Retained custom-surface cache diagnostics for one native frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeRetainedSurfaceDiagnostics {
    /// Configured retained-frame cache capacity.
    pub cache_capacity: usize,
    /// Number of retained frames currently held by the runtime.
    pub cache_entries: usize,
    /// Calls into the host bridge to render retained surfaces this frame.
    pub bridge_calls: u32,
    /// Retained surface frames reused from runtime cache this frame.
    pub cache_hits: u32,
    /// Retained surfaces the host bridge could not render this frame.
    pub miss_count: u32,
    /// Primitives encoded from retained frames this frame.
    pub retained_frame_primitive_count: usize,
    /// Text runs encoded from retained frames this frame.
    pub retained_frame_text_run_count: usize,
}

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
    /// Valid custom-shader GPU surfaces skipped by this native backend.
    pub unsupported_custom_shader_surfaces: usize,
    /// Total vertex count requested by skipped custom-shader GPU surfaces.
    pub unsupported_custom_shader_vertices: usize,
    /// Total uniform payload bytes carried by skipped custom-shader GPU surfaces.
    pub unsupported_custom_shader_uniform_bytes: usize,
    /// Total storage payload bytes carried by skipped custom-shader GPU surfaces.
    pub unsupported_custom_shader_storage_bytes: usize,
}

/// Coarse timing diagnostics for one native presentation frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeFrameTimingDiagnostics {
    /// Time spent routing a coalesced GPU-surface wheel event.
    pub coalesced_wheel_route: Duration,
    /// Time spent refreshing the runtime surface snapshot.
    pub refresh_surface: Duration,
    /// Time spent building the backend-neutral paint plan.
    pub paint_plan: Duration,
    /// Time spent rendering the scene to the cached texture.
    pub render_to_texture: Duration,
    /// Time spent encoding the full-screen blit/composite pass.
    pub full_screen_blit: Duration,
    /// Time spent refreshing the composited base frame.
    pub composited_base_refresh: Duration,
    /// Whether the composited base frame was reused from cache.
    pub composited_base_cache_hit: bool,
    /// Time spent collecting transient overlay primitives.
    pub transient_overlay_paint: Duration,
    /// Transient overlay primitive count.
    pub transient_overlay_primitives: usize,
    /// Time spent submitting GPU work and presenting the surface.
    pub submit_present: Duration,
    /// Time since the previous successful present.
    pub since_last_present: Duration,
}
