use super::SceneTextRunBuffer;
use crate::gui_runtime::native_vello::*;
use crate::runtime::PaintTextRun;

pub(super) fn encode_text(text_runs: &mut SceneTextRunBuffer, text: &PaintTextRun) {
    let align = match text.align {
        PaintTextAlign::Left => TextAlign::Left,
        PaintTextAlign::Center => TextAlign::Center,
        PaintTextAlign::Right => TextAlign::Right,
    };
    let baseline_offset = text.baseline.unwrap_or(text.font_size);
    text_runs.push(SceneTextRun {
        text: text.text.clone(),
        position: Point::new(
            text.rect.min.x,
            text.rect.min.y + baseline_offset - text.font_size,
        ),
        font_size: text.font_size,
        color: text.color,
        max_width: Some(text.rect.width().max(0.0)),
        align,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::types::{Rect, Rgba8},
        widgets::TextWrap,
    };

    #[test]
    fn encode_text_preserves_align_baseline_and_width_contract() {
        let mut text_runs = SceneTextRunBuffer::new();
        let text = PaintTextRun {
            widget_id: 7,
            text: "hello".into(),
            rect: Rect::from_min_size(Point::new(12.0, 20.0), Vector2::new(80.0, 18.0)),
            font_size: 14.0,
            color: Rgba8 {
                r: 1,
                g: 2,
                b: 3,
                a: 255,
            },
            align: PaintTextAlign::Center,
            wrap: TextWrap::None,
            baseline: Some(17.0),
        };

        encode_text(&mut text_runs, &text);

        let run = text_runs
            .first_for_test()
            .expect("text run should be staged");
        assert_eq!(run.text, "hello");
        assert_eq!(run.position, Point::new(12.0, 23.0));
        assert_eq!(run.font_size, 14.0);
        assert_eq!(run.color, text.color);
        assert_eq!(run.max_width, Some(80.0));
        assert_eq!(run.align, TextAlign::Center);
    }

    #[test]
    fn encode_text_defaults_missing_baseline_to_font_size_and_clamps_negative_width() {
        let mut text_runs = SceneTextRunBuffer::new();
        let text = PaintTextRun {
            widget_id: 8,
            text: "narrow".into(),
            rect: Rect::from_min_size(Point::new(4.0, 9.0), Vector2::new(-12.0, 18.0)),
            font_size: 13.0,
            color: Rgba8 {
                r: 10,
                g: 20,
                b: 30,
                a: 255,
            },
            align: PaintTextAlign::Right,
            wrap: TextWrap::None,
            baseline: None,
        };

        encode_text(&mut text_runs, &text);

        let run = text_runs
            .first_for_test()
            .expect("text run should be staged");
        assert_eq!(run.position, Point::new(4.0, 9.0));
        assert_eq!(run.max_width, Some(0.0));
        assert_eq!(run.align, TextAlign::Right);
    }
}
