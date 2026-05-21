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
