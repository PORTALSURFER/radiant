//! Generic selection state primitives.

use serde::{Deserialize, Serialize};

#[cfg(test)]
#[path = "selection/tests.rs"]
mod tests;

/// Three-way state for controls representing multiple selected items.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TriState {
    /// No selected items currently carry the represented value.
    #[default]
    Off,
    /// Every selected item currently carries the represented value.
    On,
    /// Selected items disagree about the represented value.
    Mixed,
}

/// Generic target for three-way selection or triage actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriageTarget {
    /// Move the selection toward the negative/left bucket.
    Negative,
    /// Move the selection toward the neutral/default bucket.
    Neutral,
    /// Move the selection toward the positive/right bucket.
    Positive,
}
