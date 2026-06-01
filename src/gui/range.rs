//! Normalized interval primitives for reusable UI models.

mod index_viewport;
mod interval;
mod scrollbar;
mod viewport;

pub use index_viewport::IndexViewport;
pub use interval::{
    NormalizedRange, NormalizedRangeParts, normalized_fraction_to_micros,
    normalized_fraction_to_milli, normalized_fraction_to_nanos,
};
pub use scrollbar::{
    NormalizedScrollbar, NormalizedScrollbarRequest, normalized_scrollbar_center_at_point,
    normalized_scrollbar_center_for_pointer, normalized_scrollbar_thumb_offset_at_point,
    normalized_scrollbar_thumb_ratio_at_point, resolve_normalized_scrollbar,
};
pub use viewport::{NormalizedPixelSnap, NormalizedViewport, NormalizedViewportParts};
