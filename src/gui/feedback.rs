//! Generic user-feedback surface primitives.

mod inline;
mod progress;
mod status;

pub use inline::{
    InlineIndicatorAnchor, InlineIndicatorLayout, InlineIndicatorMetrics, inline_indicator_layout,
    inline_indicator_reserved_width,
};
pub use progress::{
    ProgressOverlay, horizontal_discrete_meter_fill_rect, horizontal_meter_fill_rect,
    horizontal_progress_activity_rect, horizontal_progress_fill_rect,
    horizontal_progress_track_rect,
};
pub use status::{
    ConfirmPrompt, DragOverlay, HealthState, PromptIntent, RecoverySummary, StatusLineEntry,
    StatusLineEntryParts, StatusLineLog, UpdatePanel, UpdateStatus,
};
