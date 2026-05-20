mod geometry;
mod model;
mod sanitize;
#[cfg(test)]
#[path = "inline/tests.rs"]
mod tests;

pub use geometry::{inline_indicator_layout, inline_indicator_reserved_width};
pub use model::{InlineIndicatorAnchor, InlineIndicatorLayout, InlineIndicatorMetrics};
