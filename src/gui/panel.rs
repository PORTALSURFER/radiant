//! Generic panel and split-pane primitives.

use serde::{Deserialize, Serialize};

/// Stable identifier for one side of a two-pane split surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SplitPaneSlot {
    /// Upper or leading pane in the split surface.
    #[default]
    Upper,
    /// Lower or trailing pane in the split surface.
    Lower,
}

impl SplitPaneSlot {
    /// Return a small stable identifier suitable for routing and automation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Upper => "upper",
            Self::Lower => "lower",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SplitPaneSlot;

    #[test]
    fn split_pane_slot_defaults_to_upper() {
        assert_eq!(SplitPaneSlot::default(), SplitPaneSlot::Upper);
    }

    #[test]
    fn split_pane_slot_exposes_stable_routing_ids() {
        assert_eq!(SplitPaneSlot::Upper.as_str(), "upper");
        assert_eq!(SplitPaneSlot::Lower.as_str(), "lower");
    }
}
