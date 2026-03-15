//! Drag-overlay rendering for the native shell.

use super::*;

/// Render the drag banner overlay when an active drag gesture is present.
pub(in crate::gui::native_shell::state) fn render_drag_overlay(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) {
    if !model.drag_overlay.active {
        return;
    }
    let sizing = style.sizing;
    let rect = drag_overlay_rect(layout, style);
    let drag_text_layout = compute_drag_overlay_text_layout(rect, sizing);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect,
            color: style.surface_overlay,
        }),
    );
    push_border(
        primitives,
        rect,
        if model.drag_overlay.valid_target {
            style.accent_mint
        } else {
            style.accent_warning
        },
        sizing.border_width,
    );
    emit_text(
        text_runs,
        TextRun {
            text: if model.drag_overlay.target_label.is_empty() {
                model.drag_overlay.label.clone()
            } else {
                format!(
                    "{} -> {}",
                    model.drag_overlay.label, model.drag_overlay.target_label
                )
            },
            position: drag_text_layout.label.min,
            font_size: sizing.font_meta,
            color: if model.drag_overlay.valid_target {
                style.text_primary
            } else {
                style.accent_warning
            },
            max_width: Some(drag_text_layout.label.width().max(24.0)),
            align: TextAlign::Center,
        },
    );
}
