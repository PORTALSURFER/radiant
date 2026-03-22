//! State-driven overlay builders for the native shell.

use super::*;

mod browser;
mod focus;
mod waveform;

use self::{
    browser::{render_browser_tab_overlay, render_source_context_menu},
    focus::{
        render_browser_focus_overlay, render_folder_focus_overlay, render_source_focus_overlay,
        render_waveform_focus_overlay,
    },
    waveform::push_waveform_toolbar_hover_tooltip,
};

pub(super) fn push_browser_row_border(
    primitives: &mut impl PrimitiveSink,
    rect: Rect,
    color: Rgba8,
    stroke: f32,
    sides: BorderSides,
) {
    focus::push_browser_row_border(primitives, rect, color, stroke, sides);
}

pub(super) fn render_state_overlay(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let sizing = style.sizing;
    push_waveform_toolbar_hover_tooltip(primitives, text_runs, layout, style, model, shell_state);
    if let Some(visual) = shell_state.browser_search_editor_visual.clone()
        && let (Some(search_field_rect), Some(search_text_rect)) = (
            shell_state.browser_search_field_rect(layout, model),
            shell_state.browser_search_text_rect(layout, model),
        )
    {
        render_active_browser_search_editor(
            primitives,
            text_runs,
            style,
            sizing,
            search_field_rect,
            search_text_rect,
            &visual,
        );
    }
    if let Some(hovered_visible_row) = shell_state.hovered_browser_visible_row {
        let browser_rows = shell_state.cached_browser_rows(layout, style, model);
        if let Some(row) = browser_rows
            .iter()
            .find(|row| row.visible_row == hovered_visible_row)
        {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: row.rect,
                    color: browser_row_hover_fill(style),
                }),
            );
        }
    }
    if let Some(hovered_folder_row_index) = shell_state.hovered_folder_row_index {
        let folder_row_rects = shell_state.cached_folder_row_rects(layout, style, model);
        if let Some(row_rect) = folder_row_rects.get(hovered_folder_row_index) {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: *row_rect,
                    color: subtle_item_hover_fill(style),
                }),
            );
        }
    }
    if shell_state.has_focus_emphasis {
        render_waveform_focus_overlay(layout, style, model, primitives);
        render_source_focus_overlay(shell_state, layout, style, model, primitives);
        render_folder_focus_overlay(shell_state, layout, style, model, primitives, text_runs);
        render_browser_focus_overlay(shell_state, layout, style, model, primitives, text_runs);
    }
    render_browser_tab_overlay(primitives, text_runs, layout, style, model);
    render_source_context_menu(
        primitives,
        text_runs,
        layout,
        style,
        model,
        shell_state.source_context_menu,
    );
    render_options_panel(primitives, text_runs, layout, style, model);
    render_progress_overlay(primitives, text_runs, layout, style, model);
    render_confirm_prompt(primitives, text_runs, layout, style, model);
    render_drag_overlay(primitives, text_runs, layout, style, model);
}
