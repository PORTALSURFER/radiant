/// Summary for recoverable background work surfaced in a sidebar, panel, or status region.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RecoverySummary {
    /// Whether recovery work is still running in the background.
    pub in_progress: bool,
    /// Number of completed recovery entries currently visible or retained for review.
    pub entry_count: usize,
    /// Number of entries awaiting explicit user action.
    pub retained_count: usize,
}
