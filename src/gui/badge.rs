//! Generic badge and pill primitives.

mod inline;
mod model;

pub use inline::{
    InlineBadgeMetrics, InlineBadgeMetricsParts, inline_badge_cluster_reserved_width,
    inline_badge_height, inline_badge_labels, inline_badge_labels_owned,
    inline_badge_labels_owned_into, inline_badge_rects, inline_badge_rects_for_labels,
    inline_badge_rects_for_labels_into, inline_badge_rects_for_labels_with_widths_into,
    inline_badge_rects_into, inline_badge_text_origin, inline_badge_text_width, inline_badge_width,
    inline_badge_width_in_range,
};
pub use model::{
    PillEditorChoices, PillEditorInput, PillEditorPanel, PillEditorStatus, SelectablePill,
};

#[cfg(test)]
#[path = "badge/tests.rs"]
mod tests;
