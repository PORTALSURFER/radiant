/// Scene encoding counters for one native frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeSceneDiagnostics {
    /// Cumulative authoritative full-scene encodes for this native frame state.
    pub scene_encode_count: u64,
    /// Cumulative whole-scene reuse decisions for this native frame state.
    pub scene_reuse_count: u64,
    /// Cumulative authoritative retained-scene assemblies.
    pub scene_assembly_count: u64,
    /// Cumulative retained-assembly vetoes that fell back to full encoding.
    pub scene_assembly_veto_count: u64,
    /// Outcome of the most recent scene rebuild decision.
    pub scene_build_outcome: &'static str,
    /// Paint-plan traversal counters.
    pub traversal: NativeSceneTraversalDiagnostics,
    /// Text-related scene counters.
    pub text: NativeSceneTextDiagnostics,
    /// Image and SVG scene counters.
    pub media: NativeSceneMediaDiagnostics,
    /// GPU and retained/custom surface counters.
    pub surfaces: NativeSceneSurfaceDiagnostics,
}

/// Paint-plan traversal counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeSceneTraversalDiagnostics {
    /// Paint-plan primitives visited while encoding the scene.
    pub paint_plan_primitives: usize,
    /// Clip layers pushed into the scene.
    pub clip_layer_count: usize,
}

/// Text-related scene counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeSceneTextDiagnostics {
    /// Text primitives visited before batching into scene text runs.
    pub text_primitive_count: usize,
    /// Text input primitives encoded.
    pub text_input_count: usize,
    /// Total text runs submitted to the scene.
    pub text_run_count: usize,
}

/// Image and SVG scene counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeSceneMediaDiagnostics {
    /// Image primitives encoded.
    pub image_count: usize,
    /// SVG documents encoded.
    pub svg_document_count: usize,
}

/// GPU and retained/custom surface counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeSceneSurfaceDiagnostics {
    /// GPU-surface primitives visited.
    pub gpu_surface_count: usize,
    /// Retained/custom surface primitives visited.
    pub custom_surface_count: usize,
    /// Custom surfaces that fell back to placeholder rendering.
    pub custom_surface_fallback_count: u32,
}
