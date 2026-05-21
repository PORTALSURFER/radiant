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
