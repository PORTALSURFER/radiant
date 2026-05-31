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

impl WidgetStyle {
    /// Construct a style from explicit semantic tone and prominence.
    pub const fn new(tone: WidgetTone, prominence: WidgetProminence) -> Self {
        Self { tone, prominence }
    }

    /// Return this style with a different semantic tone.
    pub const fn with_tone(mut self, tone: WidgetTone) -> Self {
        self.tone = tone;
        self
    }

    /// Return this style with a different visual prominence.
    pub const fn with_prominence(mut self, prominence: WidgetProminence) -> Self {
        self.prominence = prominence;
        self
    }
}

impl Default for WidgetStyle {
    fn default() -> Self {
        Self::new(WidgetTone::Neutral, WidgetProminence::Normal)
    }
}
