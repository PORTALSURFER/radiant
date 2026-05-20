//! Frame feedback primitives shared by runtime bridges and render backends.

#[cfg(test)]
#[path = "frame/tests.rs"]
mod tests;

/// Frame-level feedback from renderer to host bridge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameBuildResult {
    /// Number of generated shape primitives.
    pub primitive_count: usize,
    /// Number of generated text runs.
    pub text_run_count: usize,
    /// Whether this redraw included a layout-driven static rebuild.
    pub layout_rebuild: bool,
    /// Whether this redraw rebuilt any static scene content.
    pub static_rebuild: bool,
    /// Whether this redraw rebuilt any state-overlay scene content.
    pub state_overlay_rebuild: bool,
    /// Whether this redraw rebuilt any motion-overlay scene content.
    pub motion_overlay_rebuild: bool,
    /// Whether runtime should keep animating while idle.
    pub needs_animation: bool,
    /// End-to-end frame time in microseconds for the redraw pass.
    pub frame_total_us: u32,
    /// Presentation duration in microseconds for the redraw pass.
    pub present_us: u32,
    /// Frame-time budget used to classify redraw jank.
    pub frame_budget_us: u32,
    /// Whether the frame exceeded the configured frame-time budget.
    pub jank: bool,
    /// Whether the redraw produced a successful surface present.
    pub presented: bool,
    /// Whether a present was expected but not completed for this redraw.
    pub missed_present: bool,
}
