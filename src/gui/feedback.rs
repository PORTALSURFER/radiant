//! Generic user-feedback surface primitives.

mod inline;
mod progress;
mod status;

pub use inline::{
    InlineIndicatorAnchor, InlineIndicatorLayout, InlineIndicatorMetrics, inline_indicator_layout,
    inline_indicator_reserved_width,
};
pub use progress::{
    ProgressOverlay, ProgressSnapshot, ProgressUpdateGate, horizontal_discrete_meter_fill_rect,
    horizontal_meter_fill_rect, horizontal_progress_activity_rect, horizontal_progress_fill_rect,
    horizontal_progress_track_rect, horizontal_value_cursor_rect,
    horizontal_value_range_edge_rects, horizontal_value_range_rect,
    horizontal_wrapped_value_range_rects, push_horizontal_value_cursor_fill,
    push_horizontal_value_range_edge_fills, push_horizontal_value_range_fill,
    vertical_bipolar_fill_rect, vertical_bipolar_value_at_point, vertical_center_track_rect,
    vertical_meter_lane_fill_rect, vertical_value_at_point, vertical_value_knob_rect,
    vertical_value_line_rect,
};
pub use status::{
    ConfirmPrompt, DragOverlay, HealthState, PromptIntent, RecoverySummary, StatusLineEntry,
    StatusLineEntryParts, StatusLineLog, UpdatePanel, UpdateStatus,
};
