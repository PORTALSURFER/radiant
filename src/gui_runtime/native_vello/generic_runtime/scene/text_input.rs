use std::sync::Arc;
use std::time::Duration;

use crate::gui::types::Rect as UiRect;
use crate::gui_runtime::native_vello::{
    text_edit::{
        SingleLineTextEditorState, TextFieldLayoutState, build_text_field_layout,
        build_text_field_layout_from_snapshot,
    },
    *,
};
use crate::widgets::TextWrap;

mod geometry;

use geometry::{caret_rect, selection_rect, text_input_geometry_is_renderable};

use super::text_input_selection::resolve_text_input_selection;

struct FocusedTextInputGeometry {
    caret_rect: UiRect,
    layout: Option<TextFieldLayoutState>,
    selection: Option<super::text_input_selection::TextInputSelectionBytes>,
}

fn focused_text_input_geometry(
    input: &PaintTextInput,
    text_renderer: &mut NativeTextRenderer,
) -> Option<FocusedTextInputGeometry> {
    if !input.focused || !text_input_geometry_is_renderable(input) {
        return None;
    }

    let text = input.state.value.as_str();
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
        input.rect.width(),
    );
    let caret_affinity = text_renderer.native_caret_affinity(input.widget_id);
    let caret_offset = if selection.has_selection {
        layout.local_x_for_byte(selection.caret_byte)
    } else if caret_affinity == CaretAffinity::Downstream {
        layout.caret_offset
    } else {
        (layout
            .snapshot
            .caret_x(selection.caret_byte, caret_affinity)
            - layout.scroll_x)
            .clamp(0.0, input.rect.width())
    };
    Some(FocusedTextInputGeometry {
        caret_rect: caret_rect(input, input.rect.min.x + caret_offset)?,
        layout: Some(layout),
        selection: Some(selection),
    })
}

#[allow(dead_code)]
fn focused_text_input_geometry_from_snapshot(
    input: &PaintTextInput,
    text_renderer: &mut NativeTextRenderer,
    snapshot: Arc<ParagraphSnapshot>,
) -> Option<FocusedTextInputGeometry> {
    if !input.focused || !text_input_geometry_is_renderable(input) {
        return None;
    }

    let text = input.state.value.as_str();
    let mut editor = SingleLineTextEditorState::collapsed_at_end(text);
    let selection =
        resolve_text_input_selection(text, input.state.caret, input.state.selection_anchor);
    editor.set_cursor(text, selection.start_byte, false);
    editor.set_cursor(text, selection.end_byte, true);
    let layout = build_text_field_layout_from_snapshot(
        snapshot,
        &mut editor,
        text,
        input.font_size,
        input.rect.width(),
    );
    let caret_affinity = text_renderer.native_caret_affinity(input.widget_id);
    let caret_offset = if selection.has_selection {
        layout.local_x_for_byte(selection.caret_byte)
    } else if caret_affinity == CaretAffinity::Downstream {
        layout.caret_offset
    } else {
        (layout
            .snapshot
            .caret_x(selection.caret_byte, caret_affinity)
            - layout.scroll_x)
            .clamp(0.0, input.rect.width())
    };
    Some(FocusedTextInputGeometry {
        caret_rect: caret_rect(input, input.rect.min.x + caret_offset)?,
        layout: Some(layout),
        selection: Some(selection),
    })
}

pub(super) fn focused_text_input_caret_rect(
    input: &PaintTextInput,
    text_renderer: &mut NativeTextRenderer,
) -> Option<UiRect> {
    focused_text_input_geometry(input, text_renderer).map(|geometry| geometry.caret_rect)
}

pub(super) fn encode_text_input(
    scene: &mut Scene,
    text_renderer: &mut NativeTextRenderer,
    input: &PaintTextInput,
    animation_time: Duration,
) {
    if !text_input_geometry_is_renderable(input) {
        return;
    }
    let text = input.state.value.as_str();
    let is_placeholder = text.is_empty();
    let display_text = if is_placeholder {
        input.placeholder.as_deref().unwrap_or_default()
    } else {
        text
    };
    let focused_geometry = input
        .focused
        .then(|| focused_text_input_geometry(input, text_renderer))
        .flatten();
    if input.focused && !is_placeholder {
        let Some(FocusedTextInputGeometry {
            caret_rect,
            layout: Some(layout),
            selection: Some(_),
        }) = focused_geometry.as_ref()
        else {
            return;
        };
        if input.selection_color.a != 0 {
            for &(start, end) in layout.selection_rects() {
                if let Some(rect) = selection_rect(input, start, end) {
                    super::encode_rect(scene, input.selection_color, rect);
                }
            }
        }
        encode_block_caret(scene, input, *caret_rect, animation_time);
        draw_text_input_layout(scene, text_renderer, input, layout, input.color);
        draw_completion_suffix(
            scene,
            text_renderer,
            input,
            layout.local_x_for_byte(text.len()),
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
        if let Some(geometry) = focused_geometry {
            encode_block_caret(scene, input, geometry.caret_rect, animation_time);
        }
        if !is_placeholder {
            let suffix_x = text_renderer
                .layout_text(display_text, input.font_size)
                .map(|layout| layout.width)
                .unwrap_or(0.0);
            draw_completion_suffix(scene, text_renderer, input, suffix_x);
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
    text_renderer.draw_text_run(
        scene,
        text,
        TextRunParts {
            position: Point::new(
                input.rect.min.x,
                input.rect.min.y + baseline_offset - input.font_size,
            ),
            font_size: input.font_size,
            color,
            max_width: Some(input.rect.width().max(0.0)),
            align: TextAlign::Left,
            wrap: TextWrap::None,
        },
    );
}

#[allow(dead_code)]
fn draw_text_input_value_from_snapshot(
    scene: &mut Scene,
    text_renderer: &mut NativeTextRenderer,
    input: &PaintTextInput,
    snapshot: Arc<ParagraphSnapshot>,
) {
    if input.state.value.is_empty() {
        return;
    }
    let baseline_offset = input.baseline.unwrap_or(input.font_size);
    text_renderer.draw_paragraph_snapshot(
        scene,
        &snapshot,
        TextSnapshotPaint {
            position: Point::new(
                input.rect.min.x,
                input.rect.min.y + baseline_offset - input.font_size,
            ),
            font_size: input.font_size,
            color: input.color,
            clip_width: input.rect.width().max(0.0),
            scroll_x: 0.0,
        },
    );
    draw_completion_suffix(scene, text_renderer, input, snapshot.width);
}

fn draw_text_input_layout(
    scene: &mut Scene,
    text_renderer: &mut NativeTextRenderer,
    input: &PaintTextInput,
    layout: &TextFieldLayoutState,
    color: Rgba8,
) {
    let baseline_offset = input.baseline.unwrap_or(input.font_size);
    text_renderer.draw_paragraph_snapshot(
        scene,
        &layout.snapshot,
        TextSnapshotPaint {
            position: Point::new(
                input.rect.min.x,
                input.rect.min.y + baseline_offset - input.font_size,
            ),
            font_size: input.font_size,
            color,
            clip_width: input.rect.width().max(0.0),
            scroll_x: layout.scroll_x,
        },
    );
}

fn draw_completion_suffix(
    scene: &mut Scene,
    text_renderer: &mut NativeTextRenderer,
    input: &PaintTextInput,
    x: f32,
) {
    let Some(suffix) = input
        .completion_suffix
        .as_ref()
        .filter(|suffix| !suffix.is_empty())
    else {
        return;
    };
    if input.state.value.is_empty() || !x.is_finite() {
        return;
    }
    let gap = (input.font_size * 0.14).clamp(1.0, 3.0);
    let suffix_x = input.rect.min.x + x + gap;
    let max_width = input.rect.max.x - suffix_x;
    if max_width <= 0.0 {
        return;
    }
    let baseline_offset = input.baseline.unwrap_or(input.font_size);
    text_renderer.draw_text_run(
        scene,
        suffix.as_ref(),
        TextRunParts {
            position: Point::new(
                suffix_x,
                input.rect.min.y + baseline_offset - input.font_size,
            ),
            font_size: input.font_size,
            color: input.completion_color,
            max_width: Some(max_width),
            align: TextAlign::Left,
            wrap: TextWrap::None,
        },
    );
}

fn encode_block_caret(
    scene: &mut Scene,
    input: &PaintTextInput,
    caret_rect: UiRect,
    animation_time: Duration,
) {
    if input.caret_color.a == 0 || !caret_rect.has_finite_positive_area() {
        return;
    }
    let pulse = (animation_time.as_secs_f32() * std::f32::consts::TAU * 0.85).sin();
    let alpha = (0.42 + 0.28 * ((pulse + 1.0) * 0.5)).clamp(0.0, 1.0);
    let mut color = input.caret_color;
    color.a = ((color.a as f32) * alpha).round() as u8;
    super::encode_rect(scene, color, caret_rect);
}

#[cfg(test)]
mod tests {
    use super::{encode_block_caret, focused_text_input_caret_rect, geometry::caret_rect};
    use crate::{
        gui::types::{Point, Rect, Rgba8},
        gui_runtime::native_vello::{CaretAffinity, NativeTextRenderer},
        runtime::PaintTextInput,
        widgets::TextInputState,
    };
    use std::time::Duration;
    use vello::Scene;

    #[test]
    fn zero_alpha_caret_color_skips_native_caret_geometry() {
        let input = PaintTextInput {
            widget_id: 1,
            rect: Rect::from_min_max(Point::default(), Point::new(120.0, 28.0)),
            placeholder: None,
            completion_suffix: None,
            state: TextInputState::from_value(String::from("text")),
            font_size: 12.0,
            baseline: None,
            color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            placeholder_color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            completion_color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            selection_color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 0,
            },
            caret_color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 0,
            },
            focused: true,
        };
        let mut scene = Scene::new();

        encode_block_caret(
            &mut scene,
            &input,
            caret_rect(&input, 12.0).expect("finite caret geometry"),
            Duration::ZERO,
        );

        assert!(scene.encoding().is_empty());
    }

    #[test]
    fn focused_text_input_caret_area_projects_empty_text() {
        let input = focused_input(
            "",
            0,
            0,
            Rect::from_min_max(Point::new(8.0, 10.0), Point::new(160.0, 38.0)),
        );
        let mut text_renderer = NativeTextRenderer::new();

        let caret = focused_text_input_caret_rect(&input, &mut text_renderer)
            .expect("empty focused text should have candidate geometry");

        assert_eq!(caret.min.x, input.rect.min.x);
        assert!(caret.has_finite_positive_area());
    }

    #[test]
    fn focused_text_input_caret_area_projects_unicode_and_selection_caret() {
        let bounds = Rect::from_min_max(Point::new(8.0, 10.0), Point::new(260.0, 38.0));
        let mut text_renderer = NativeTextRenderer::new();
        let at_start = focused_input("aé日", 0, 0, bounds);
        let after_unicode = focused_input("aé日", 2, 2, bounds);
        let selected = focused_input("abcdef", 2, 5, bounds);
        let collapsed_at_caret = focused_input("abcdef", 2, 2, bounds);

        let start_x = focused_text_input_caret_rect(&at_start, &mut text_renderer)
            .expect("Unicode input start should project")
            .min
            .x;
        let unicode_x = focused_text_input_caret_rect(&after_unicode, &mut text_renderer)
            .expect("Unicode input caret should project")
            .min
            .x;
        let selected_x = focused_text_input_caret_rect(&selected, &mut text_renderer)
            .expect("selected input caret should project")
            .min
            .x;
        let collapsed_x = focused_text_input_caret_rect(&collapsed_at_caret, &mut text_renderer)
            .expect("collapsed input caret should project")
            .min
            .x;

        assert!(unicode_x > start_x);
        assert_eq!(selected_x, collapsed_x);
    }

    #[test]
    fn focused_text_input_caret_geometry_preserves_bidi_affinity() {
        let input = focused_input(
            "שלום world",
            1,
            1,
            Rect::from_min_max(Point::new(8.0, 10.0), Point::new(260.0, 38.0)),
        );
        let mut text_renderer = NativeTextRenderer::new();
        text_renderer.set_native_caret_affinity(input.widget_id, CaretAffinity::Upstream);
        let upstream = focused_text_input_caret_rect(&input, &mut text_renderer)
            .expect("upstream caret should project")
            .min
            .x;
        text_renderer.set_native_caret_affinity(input.widget_id, CaretAffinity::Downstream);
        let downstream = focused_text_input_caret_rect(&input, &mut text_renderer)
            .expect("downstream caret should project")
            .min
            .x;
        assert_ne!(upstream, downstream);
    }

    #[test]
    fn focused_text_input_caret_area_ignores_hidden_caret_alpha() {
        let mut input = focused_input(
            "text",
            4,
            4,
            Rect::from_min_max(Point::new(8.0, 10.0), Point::new(160.0, 38.0)),
        );
        input.caret_color.a = 0;
        let mut text_renderer = NativeTextRenderer::new();

        assert!(focused_text_input_caret_rect(&input, &mut text_renderer).is_some());
    }

    #[test]
    fn focused_text_input_caret_area_clamps_long_text_and_rejects_malformed_geometry() {
        let mut text_renderer = NativeTextRenderer::new();
        let long = focused_input(
            "a very long candidate string that must scroll",
            45,
            45,
            Rect::from_min_max(Point::new(8.0, 10.0), Point::new(36.0, 38.0)),
        );
        let long_caret = focused_text_input_caret_rect(&long, &mut text_renderer)
            .expect("long text should clamp to the field");
        assert!(long_caret.min.x >= long.rect.min.x);
        assert!(long_caret.max.x <= long.rect.max.x);

        let mut malformed = focused_input(
            "text",
            4,
            4,
            Rect::from_min_max(Point::new(8.0, 10.0), Point::new(8.0, 38.0)),
        );
        assert_eq!(
            focused_text_input_caret_rect(&malformed, &mut text_renderer),
            None
        );
        malformed.rect.max.x = f32::NAN;
        assert_eq!(
            focused_text_input_caret_rect(&malformed, &mut text_renderer),
            None
        );
    }

    fn focused_input(
        text: &str,
        caret: usize,
        selection_anchor: usize,
        rect: Rect,
    ) -> PaintTextInput {
        PaintTextInput {
            widget_id: 1,
            rect,
            placeholder: None,
            completion_suffix: None,
            state: TextInputState {
                value: text.to_owned(),
                caret,
                selection_anchor,
            },
            font_size: 14.0,
            baseline: None,
            color: Rgba8::default(),
            placeholder_color: Rgba8::default(),
            completion_color: Rgba8::default(),
            selection_color: Rgba8::default(),
            caret_color: Rgba8::default(),
            focused: true,
        }
    }
}
