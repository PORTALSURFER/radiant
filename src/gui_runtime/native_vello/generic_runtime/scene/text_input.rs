use std::time::Duration;

use crate::gui_runtime::native_vello::{
    text_edit::{SingleLineTextEditorState, build_text_field_layout},
    *,
};

use super::text_input_selection::resolve_text_input_selection;

pub(super) fn encode_text_input(
    scene: &mut Scene,
    text_renderer: &mut NativeTextRenderer,
    input: &PaintTextInput,
    animation_time: Duration,
) {
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
            text,
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

fn caret_size(input: &PaintTextInput) -> Option<(f32, f32)> {
    if !font_size_is_renderable(input.font_size)
        || !input.rect.min.x.is_finite()
        || !input.rect.min.y.is_finite()
        || !input.rect.max.x.is_finite()
        || !input.rect.max.y.is_finite()
    {
        return None;
    }
    let max_width = input.rect.width().max(0.0);
    let max_height = input.rect.height().max(0.0);
    if max_width <= 0.0 || max_height <= 0.0 {
        return None;
    }
    let caret_width = (input.font_size * 0.62).clamp(7.0, 12.0).min(max_width);
    let caret_height = (input.font_size * 1.15).clamp(12.0, 24.0).min(max_height);
    Some((caret_width, caret_height))
}

fn selection_rect(input: &PaintTextInput, start: f32, end: f32) -> Option<UiRect> {
    if end <= start || input.rect.max.x <= input.rect.min.x || input.rect.max.y <= input.rect.min.y
    {
        return None;
    }
    let start_x = (input.rect.min.x + start).clamp(input.rect.min.x, input.rect.max.x);
    let end_x = (input.rect.min.x + end).clamp(input.rect.min.x, input.rect.max.x);
    if end_x <= start_x {
        return None;
    }
    Some(UiRect::from_min_max(
        Point::new(start_x, input.rect.min.y),
        Point::new(end_x, input.rect.max.y),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{gui::types::Rect, widgets::TextInputState};

    const WHITE: Rgba8 = Rgba8 {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };

    #[test]
    fn caret_size_clamps_to_cramped_text_input_bounds() {
        let input = cramped_text_input();

        assert_eq!(caret_size(&input), Some((5.0, 7.0)));
    }

    #[test]
    fn selection_rect_remains_visible_for_cramped_text_input_bounds() {
        let input = cramped_text_input();

        let rect = selection_rect(&input, 1.0, 4.0).expect("visible selection rect");

        assert_eq!(rect.min.y, input.rect.min.y);
        assert_eq!(rect.max.y, input.rect.max.y);
        assert!(rect.width() > 0.0);
    }

    #[test]
    fn caret_size_rejects_invalid_text_input_geometry() {
        let mut input = cramped_text_input();

        input.font_size = f32::NAN;
        assert_eq!(caret_size(&input), None);

        input.font_size = 10.0;
        input.rect.max.x = f32::INFINITY;
        assert_eq!(caret_size(&input), None);
    }

    fn cramped_text_input() -> PaintTextInput {
        PaintTextInput {
            widget_id: 1,
            rect: Rect::from_min_max(Point::new(0.0, 0.0), Point::new(5.0, 7.0)),
            placeholder: None,
            state: TextInputState {
                value: String::from("ml"),
                caret: 2,
                selection_anchor: 2,
            },
            font_size: 10.0,
            baseline: None,
            color: WHITE,
            placeholder_color: WHITE,
            selection_color: WHITE,
            caret_color: WHITE,
            focused: true,
        }
    }
}
