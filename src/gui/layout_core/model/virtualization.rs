//! Virtualization policy values for large scrollable child lists.

/// Scroll virtualization axis selection.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VirtualizationAxis {
    /// Virtualize children along the horizontal axis.
    Horizontal,
    /// Virtualize children along the vertical axis.
    Vertical,
}

/// Optional virtualization policy for large scrollable child lists.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VirtualizationPolicy {
    /// Enables virtualization when `true`.
    pub enabled: bool,
    /// Main-axis used to compute visible windows.
    pub axis: VirtualizationAxis,
    /// Extra pixels before/after the viewport to pre-materialize.
    pub overscan_px: f32,
}
