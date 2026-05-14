/// Collected traversal counters for one layout evaluation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LayoutStats {
    /// Number of nodes measured with a cache miss.
    pub measured_nodes: usize,
    /// Number of nodes visited by the layout traversal.
    pub laid_out_nodes: usize,
    /// Number of nodes materialized into `LayoutOutput::rects`.
    pub materialized_nodes: usize,
}
