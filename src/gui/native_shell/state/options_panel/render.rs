//! Primitive/text rendering for the native-shell options panel.

use super::actions::options_panel_button_text;
use super::geometry::options_panel_layout;
use super::style::{
    inset_rect, status_options_button_border, status_options_button_fill,
    status_options_button_icon_color,
};
use super::*;

pub(super) fn render_status_options_button(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    sizing: SizingTokens,
    button_rect: Rect,
    hovered: bool,
    flashed: bool,
    motion_wave: f32,
) {
    let fill = status_options_button_fill(style, hovered, flashed, motion_wave);
    let border = status_options_button_border(style, hovered, flashed, motion_wave);
    let icon_color = status_options_button_icon_color(style, hovered, flashed, motion_wave);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: button_rect,
            color: fill,
        }),
    );
    push_border(primitives, button_rect, border, sizing.border_width);
    let icon_rect = inset_rect(
        button_rect,
        sizing.text_inset_x.max(3.0),
        sizing.text_inset_y.max(2.0),
    );
    let _ = emit_toolbar_svg_icon(primitives, WaveformToolbarIcon::Cog, icon_rect, icon_color);
}

pub(super) fn render_options_panel(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) {
    let Some(panel) = options_panel_layout(layout, style, model) else {
        return;
    };
    let sizing = style.sizing;
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: panel.panel_rect,
            color: style.surface_overlay,
        }),
    );
    push_border(
        primitives,
        panel.panel_rect,
        blend_color(style.border_emphasis, style.highlight_orange, 0.42),
        sizing.border_width,
    );
    emit_text(
        text_runs,
        TextRun {
            text: String::from("Options"),
            position: panel.title_rect.min,
            font_size: sizing.font_title,
            color: style.text_primary,
            max_width: Some(panel.title_rect.width().max(36.0)),
            align: TextAlign::Left,
        },
    );
    for button in &panel.buttons {
        let label_rect = compute_action_button_text_rect(button.rect, sizing);
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: button.rect,
                color: style.surface_base,
            }),
        );
        push_border(
            primitives,
            button.rect,
            blend_color(style.border_emphasis, style.text_primary, 0.18),
            sizing.border_width,
        );
        emit_text(
            text_runs,
            TextRun {
                text: options_panel_button_text(button.label, model),
                position: label_rect.min,
                font_size: sizing.font_meta,
                color: if button.label == "YOLO Edits" {
                    style.accent_warning
                } else {
                    button.text_color
                },
                max_width: Some(label_rect.width().max(12.0)),
                align: TextAlign::Left,
            },
        );
    }
}
