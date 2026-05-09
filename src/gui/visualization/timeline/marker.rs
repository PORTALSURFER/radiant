use crate::gui::range::NormalizedRange;

/// Generic marker preview for a normalized timeline or signal visualization.
///
/// The range is expressed in normalized milli, micro, and nano precision so
/// hosts can project markers into deeply zoomed timelines without losing
/// pointer precision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimelineMarkerPreview {
    /// Marker range in normalized timeline precision.
    pub range: NormalizedRange,
    /// Whether this marker is currently selected for edit operations.
    pub selected: bool,
    /// Whether this marker is focused for keyboard review.
    pub focused: bool,
}
