/// Generic health state for compact status chips and panel summaries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HealthState {
    /// The represented subsystem is available and behaving as expected.
    #[default]
    Healthy,
    /// The represented subsystem is unavailable, degraded, or reporting an error.
    Error,
}
