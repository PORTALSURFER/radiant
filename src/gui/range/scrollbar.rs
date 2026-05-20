mod geometry;
mod model;

#[cfg(test)]
#[path = "scrollbar/tests.rs"]
mod tests;

pub use geometry::{
    normalized_scrollbar_center_at_point, normalized_scrollbar_center_for_pointer,
    normalized_scrollbar_thumb_offset_at_point, normalized_scrollbar_thumb_ratio_at_point,
    resolve_normalized_scrollbar,
};
pub use model::{NormalizedScrollbar, NormalizedScrollbarRequest};
