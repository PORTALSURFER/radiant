use crate::gui::types::Rgba8;

use crate::widgets::{WidgetId, WidgetSizing};

/// Progress-bar interaction output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgressBarMessage {
    /// The progress bar was activated by its configured pointer interaction.
    Activate,
}

/// Horizontal progress mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProgressBarMode {
    /// Leading fill for known progress in `0.0..=1.0`.
    Determinate(f32),
    /// Moving activity segment for unknown progress.
    Indeterminate(f32),
}

/// Immutable progress-bar paint and interaction configuration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProgressBarProps {
    /// Progress mode.
    pub mode: ProgressBarMode,
    /// Optional explicit track color. Defaults to the current theme.
    pub track_color: Option<Rgba8>,
    /// Optional explicit fill color. Defaults to the current theme.
    pub fill_color: Option<Rgba8>,
    /// Maximum painted track height inside the widget bounds.
    pub max_track_height: f32,
    /// Width of the indeterminate segment as a fraction of the track.
    pub activity_segment_fraction: f32,
    /// Minimum width of the indeterminate segment in logical pixels.
    pub min_activity_segment_width: f32,
    /// Whether primary pointer activation emits [`ProgressBarMessage::Activate`].
    pub interactive: bool,
}

/// Named construction fields for [`super::ProgressBarWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct ProgressBarWidgetParts {
    /// Stable widget identity used by layout, input, and paint.
    pub id: WidgetId,
    /// Intrinsic progress-bar sizing contract.
    pub sizing: WidgetSizing,
    /// Progress-bar configuration.
    pub props: ProgressBarProps,
}

impl ProgressBarMode {
    fn normalized(self) -> Self {
        match self {
            Self::Determinate(fraction) => Self::Determinate(normalized_fraction(fraction)),
            Self::Indeterminate(position) => {
                if position.is_finite() {
                    Self::Indeterminate(position.rem_euclid(1.0))
                } else {
                    Self::Indeterminate(0.0)
                }
            }
        }
    }
}

impl ProgressBarProps {
    /// Build default progress-bar configuration for the given mode.
    pub fn new(mode: ProgressBarMode) -> Self {
        Self {
            mode: mode.normalized(),
            track_color: None,
            fill_color: None,
            max_track_height: 8.0,
            activity_segment_fraction: 0.32,
            min_activity_segment_width: 1.0,
            interactive: false,
        }
    }

    pub(super) fn normalized(self) -> Self {
        Self {
            mode: self.mode.normalized(),
            max_track_height: finite_nonnegative(self.max_track_height),
            activity_segment_fraction: normalized_fraction(self.activity_segment_fraction),
            min_activity_segment_width: finite_nonnegative(self.min_activity_segment_width),
            ..self
        }
    }
}

fn normalized_fraction(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}
