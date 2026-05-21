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
