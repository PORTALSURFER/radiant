/// Current state of a resource slot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ResourceLoadState {
    /// No load has been requested.
    #[default]
    Idle,
    /// A background load is running.
    Loading,
    /// The latest load completed successfully.
    Ready,
    /// The latest load failed.
    Failed,
}
