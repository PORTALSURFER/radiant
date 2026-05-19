use std::time::Duration;

use crate::gui_runtime::native_vello::{
    text_edit::{SingleLineTextEditorState, build_text_field_layout},
    *,
};

mod geometry;

use geometry::{caret_size, selection_rect, text_input_geometry_is_renderable};

use super::text_input_selection::resolve_text_input_selection;

pub(super) fn encode_text_input(
    scene: &mut Scene,
    text_renderer: &mut NativeTextRenderer,
    input: &PaintTextInput,
    animation_time: Duration,
) {
    if !text_input_geometry_is_renderable(input) {
        return;
    }
    let text_rect = input.rect;
    let text = input.state.value.as_str();
    let is_placeholder = text.is_empty();
    let display_text = if is_placeholder {
        input.placeholder.as_deref().unwrap_or_default()
    } else {
        text
    };
    if input.focused && !is_placeholder {
        let mut editor = SingleLineTextEditorState::collapsed_at_end(text);
        let selection =
            resolve_text_input_selection(text, input.state.caret, input.state.selection_anchor);
        editor.set_cursor(text, selection.start_byte, false);
        editor.set_cursor(text, selection.end_byte, true);
        let layout = build_text_field_layout(
            text_renderer,
            &mut editor,
            text,
            input.font_size,
            text_rect.width(),
        );
        if let Some((start, end)) = layout.selection_offsets
            && let Some(rect) = selection_rect(input, start, end)
        {
            super::encode_rect(scene, input.selection_color, rect);
        }
        let caret_offset = if selection.has_selection {
            layout.local_x_for_byte(selection.caret_byte)
        } else {
            layout.caret_offset
        };
        encode_block_caret(scene, input, text_rect.min.x + caret_offset, animation_time);
        draw_text_input_text(
            scene,
            text_renderer,
            input,
            layout.visible_text(text),
            input.color,
        );
    } else {
        draw_text_input_text(
            scene,
            text_renderer,
            input,
            display_text,
            if is_placeholder {
                input.placeholder_color
            } else {
                input.color
            },
        );
        if input.focused {
            encode_block_caret(scene, input, text_rect.min.x, animation_time);
        }
    }
}

fn draw_text_input_text(
    scene: &mut Scene,
    text_renderer: &mut NativeTextRenderer,
    input: &PaintTextInput,
    text: &str,
    color: Rgba8,
) {
    if text.is_empty() {
        return;
    }
    let baseline_offset = input.baseline.unwrap_or(input.font_size);
    text_renderer.draw_scene_text_runs(
        scene,
        [SceneTextRun {
            text: text.into(),
            position: Point::new(
                input.rect.min.x,
                input.rect.min.y + baseline_offset - input.font_size,
            ),
            font_size: input.font_size,
            color,
            max_width: Some(input.rect.width().max(0.0)),
            align: TextAlign::Left,
        }],
    );
}

fn encode_block_caret(scene: &mut Scene, input: &PaintTextInput, x: f32, animation_time: Duration) {
    if !x.is_finite() {
        return;
    }
    let pulse = (animation_time.as_secs_f32() * std::f32::consts::TAU * 0.85).sin();
    let alpha = (0.42 + 0.28 * ((pulse + 1.0) * 0.5)).clamp(0.0, 1.0);
    let mut color = input.caret_color;
    color.a = ((color.a as f32) * alpha).round() as u8;
    let Some((caret_width, caret_height)) = caret_size(input) else {
        return;
    };
    let caret_y = input.rect.min.y + (input.rect.height() - caret_height) * 0.5;
    let caret_x = x.clamp(
        input.rect.min.x,
        (input.rect.max.x - caret_width).max(input.rect.min.x),
    );
    super::encode_rect(
        scene,
        color,
        UiRect::from_min_max(
            Point::new(caret_x, caret_y),
            Point::new(caret_x + caret_width, caret_y + caret_height),
        ),
    );
}
