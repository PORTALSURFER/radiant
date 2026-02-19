//! Slotized helpers for prompt/progress/drag overlay geometry.

mod drag;
mod progress;
mod prompt;
mod shared;

use super::super::style::SizingTokens;
use crate::gui::types::Rect;

/// Slot-resolved prompt overlay geometry for hit-testing and rendering.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PromptOverlaySections {
    pub dialog: Rect,
    pub confirm_button: Rect,
    pub cancel_button: Rect,
    pub input: Option<Rect>,
}

/// Slot-resolved progress overlay geometry for hit-testing and rendering.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ProgressOverlaySections {
    pub dialog: Rect,
    pub progress_bar: Rect,
    pub cancel_button: Rect,
}

/// Compute prompt overlay dialog/button/input sections inside content bounds.
pub(crate) fn compute_prompt_overlay_sections(
    content: Rect,
    sizing: SizingTokens,
    has_input: bool,
    has_target_label: bool,
) -> PromptOverlaySections {
    prompt::compute_prompt_overlay_sections(content, sizing, has_input, has_target_label)
}

/// Compute progress overlay dialog/progress bar/cancel sections inside content bounds.
pub(crate) fn compute_progress_overlay_sections(
    content: Rect,
    sizing: SizingTokens,
    modal: bool,
) -> ProgressOverlaySections {
    progress::compute_progress_overlay_sections(content, sizing, modal)
}

/// Compute drag overlay banner rect between content and status bars.
pub(crate) fn compute_drag_overlay_rect(
    content: Rect,
    status_bar: Rect,
    sizing: SizingTokens,
) -> Rect {
    drag::compute_drag_overlay_rect(content, status_bar, sizing)
}
