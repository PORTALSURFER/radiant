//! Widget semantic style vocabulary independent from application themes.

/// Shared style vocabulary that avoids app-specific shell naming.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WidgetTone {
    /// Default neutral surface or text treatment.
    Neutral,
    /// Stronger emphasis for primary actions or highlighted content.
    Accent,
    /// Positive or successful confirmation state.
    Success,
    /// Cautionary or warning state.
    Warning,
    /// Destructive or dangerous action state.
    Danger,
}

/// Shared prominence vocabulary for widget styling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WidgetProminence {
    /// Low-chrome treatment that lets surrounding content dominate.
    Subtle,
    /// Standard control treatment.
    Normal,
    /// High-emphasis treatment for primary actions or critical affordances.
    Strong,
}

/// Minimal widget style contract independent from application-specific themes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WidgetStyle {
    /// Semantic tone used to resolve colors and accents.
    pub tone: WidgetTone,
    /// Relative visual weight of the widget chrome.
    pub prominence: WidgetProminence,
}

impl Default for WidgetStyle {
    fn default() -> Self {
        Self {
            tone: WidgetTone::Neutral,
            prominence: WidgetProminence::Normal,
        }
    }
}
