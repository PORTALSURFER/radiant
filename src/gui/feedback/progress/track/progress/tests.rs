use super::{
    horizontal_progress_activity_rect, horizontal_progress_fill_rect,
    horizontal_progress_track_rect, horizontal_value_cursor_rect,
    horizontal_value_range_edge_rects, horizontal_value_range_rect,
    horizontal_wrapped_value_range_rects, push_horizontal_progress_fill,
    push_horizontal_value_cursor_fill, push_horizontal_value_cursor_fills,
    push_horizontal_value_range_edge_fills, push_horizontal_value_range_fill,
};
use crate::gui::types::{Point, Rect};
use crate::{
    gui::types::Rgba8,
    runtime::{PaintPrimitive, WidgetPaint},
};

#[path = "tests/cursor.rs"]
mod cursor;
#[path = "tests/edge.rs"]
mod edge;
#[path = "tests/range.rs"]
mod range;
#[path = "tests/scalar.rs"]
mod scalar;
