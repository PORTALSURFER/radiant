//! Inline badge text measurement and row geometry helpers.

mod geometry;
mod labels;
mod metrics;

pub use geometry::{
    inline_badge_cluster_reserved_width, inline_badge_height, inline_badge_rects,
    inline_badge_rects_for_labels, inline_badge_rects_for_labels_into, inline_badge_rects_into,
    inline_badge_text_origin, inline_badge_text_width, inline_badge_width,
};
pub use labels::{inline_badge_labels, inline_badge_labels_owned, inline_badge_labels_owned_into};
pub use metrics::{InlineBadgeMetrics, InlineBadgeMetricsParts};
