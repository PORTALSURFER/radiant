/// Single-use feedback event tokens for a normalized timeline.
///
/// Hosts increment these counters when user-visible operations complete or
/// fail. Radiant renderers can compare tokens across frames and show transient
/// feedback without owning host-specific timestamps, operation names, or domain
/// workflows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct TimelineFeedbackEvents {
    /// Token for the primary successful timeline operation.
    pub primary_success_nonce: u64,
    /// Token for the primary failed timeline operation.
    pub primary_failure_nonce: u64,
    /// Token for a secondary successful timeline operation.
    pub secondary_success_nonce: u64,
}

impl TimelineFeedbackEvents {
    /// Build timeline feedback events from explicit monotonic tokens.
    pub fn new(
        primary_success_nonce: u64,
        primary_failure_nonce: u64,
        secondary_success_nonce: u64,
    ) -> Self {
        Self {
            primary_success_nonce,
            primary_failure_nonce,
            secondary_success_nonce,
        }
    }
}
