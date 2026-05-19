use crate::gui_runtime::native_vello::*;

pub(super) fn caret_size(input: &PaintTextInput) -> Option<(f32, f32)> {
    if !text_input_geometry_is_renderable(input) {
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

pub(super) fn selection_rect(input: &PaintTextInput, start: f32, end: f32) -> Option<UiRect> {
    if !text_input_geometry_is_renderable(input)
        || !start.is_finite()
        || !end.is_finite()
        || end <= start
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

pub(super) fn text_input_geometry_is_renderable(input: &PaintTextInput) -> bool {
    font_size_is_renderable(input.font_size) && input.rect.has_finite_positive_area()
}

#[cfg(test)]
mod tests {
    use super::{caret_size, selection_rect, text_input_geometry_is_renderable};
    use crate::{
        gui::types::Rect,
        gui_runtime::native_vello::{PaintTextInput, Point, Rgba8},
        widgets::TextInputState,
    };

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

    #[test]
    fn selection_rect_rejects_invalid_offsets_and_geometry() {
        let mut input = cramped_text_input();

        assert_eq!(selection_rect(&input, f32::NAN, 4.0), None);
        assert_eq!(selection_rect(&input, 4.0, f32::INFINITY), None);

        input.rect.max.x = f32::NAN;
        assert_eq!(selection_rect(&input, 1.0, 4.0), None);
    }

    #[test]
    fn text_input_geometry_requires_renderable_font_and_rect() {
        let mut input = cramped_text_input();

        assert!(text_input_geometry_is_renderable(&input));

        input.font_size = 0.0;
        assert!(!text_input_geometry_is_renderable(&input));

        input.font_size = 10.0;
        input.rect = Rect::from_min_max(Point::new(4.0, 0.0), Point::new(2.0, 7.0));
        assert!(!text_input_geometry_is_renderable(&input));
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
