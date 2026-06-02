//! Feedback, status, and progress geometry prelude exports.

pub use crate::gui::feedback::{
    ProgressSnapshot, ProgressUpdateGate, StatusLineEntry, StatusLineEntryParts, StatusLineLog,
    ThrottledProgressReporter, horizontal_progress_fill_rect, horizontal_value_cursor_rect,
    horizontal_value_range_edge_rects, horizontal_value_range_rect,
    horizontal_wrapped_value_range_rects, push_horizontal_value_cursor_fill,
    push_horizontal_value_range_edge_fills, push_horizontal_value_range_fill,
    vertical_bipolar_fill_rect, vertical_bipolar_value_at_point, vertical_center_track_rect,
    vertical_meter_lane_fill_rect, vertical_value_at_point, vertical_value_knob_rect,
    vertical_value_line_rect,
};
