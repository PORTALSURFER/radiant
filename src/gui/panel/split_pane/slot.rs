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

    /// Select the value associated with this split-pane slot.
    pub fn select<'a, T>(self, upper: &'a T, lower: &'a T) -> &'a T {
        match self {
            Self::Upper => upper,
            Self::Lower => lower,
        }
    }

    /// Select the mutable value associated with this split-pane slot.
    pub fn select_mut<'a, T>(self, upper: &'a mut T, lower: &'a mut T) -> &'a mut T {
        match self {
            Self::Upper => upper,
            Self::Lower => lower,
        }
    }
}
