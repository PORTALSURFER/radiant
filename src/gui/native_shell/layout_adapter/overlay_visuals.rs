//! Slotized visual geometry for prompt/progress/drag overlays.

use super::overlays::{
    ProgressOverlaySections, PromptOverlaySections, compute_drag_overlay_rect,
    compute_progress_overlay_sections, compute_prompt_overlay_sections,
};
use crate::gui::feedback::horizontal_progress_fill_rect;
use crate::gui::native_shell::style::SizingTokens;
use crate::gui::types::Rect;

/// Slot-resolved visual geometry for the confirmation prompt overlay.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PromptOverlayVisualLayout {
    pub scrim: Rect,
    pub sections: PromptOverlaySections,
}

/// Slot-resolved visual geometry for the progress overlay.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ProgressOverlayVisualLayout {
    pub scrim: Option<Rect>,
    pub sections: ProgressOverlaySections,
    pub progress_fill: Option<Rect>,
}

/// Slot-resolved visual geometry for the drag overlay banner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DragOverlayVisualLayout {
    pub banner: Rect,
}

/// Compute prompt overlay visual geometry used by rendering and hit-testing.
pub(crate) fn compute_prompt_overlay_visual_layout(
    root: Rect,
    content: Rect,
    sizing: SizingTokens,
    has_input: bool,
    has_target_label: bool,
) -> PromptOverlayVisualLayout {
    PromptOverlayVisualLayout {
        scrim: root,
        sections: compute_prompt_overlay_sections(content, sizing, has_input, has_target_label),
    }
}

/// Compute progress overlay visual geometry, including filled-track rect.
pub(crate) fn compute_progress_overlay_visual_layout(
    root: Rect,
    content: Rect,
    sizing: SizingTokens,
    modal: bool,
    progress_fraction: f32,
) -> ProgressOverlayVisualLayout {
    let sections = compute_progress_overlay_sections(content, sizing, modal);
    ProgressOverlayVisualLayout {
        scrim: modal.then_some(root),
        sections,
        progress_fill: horizontal_progress_fill_rect(sections.progress_bar, progress_fraction),
    }
}

/// Compute drag overlay visual geometry used by rendering and hit-testing.
pub(crate) fn compute_drag_overlay_visual_layout(
    content: Rect,
    status_bar: Rect,
    sizing: SizingTokens,
) -> DragOverlayVisualLayout {
    DragOverlayVisualLayout {
        banner: compute_drag_overlay_rect(content, status_bar, sizing),
    }
}
