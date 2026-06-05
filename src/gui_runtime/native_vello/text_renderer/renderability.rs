use super::SceneTextRun;
use crate::gui::types::Point;

pub(super) fn text_run_is_renderable(run: &SceneTextRun) -> bool {
    text_run_parts_are_renderable(
        run.text.as_ref(),
        run.position,
        run.font_size,
        run.max_width,
    )
}

pub(super) fn text_run_parts_are_renderable(
    text: &str,
    position: Point,
    font_size: f32,
    max_width: Option<f32>,
) -> bool {
    !text.is_empty()
        && font_size_is_renderable(font_size)
        && position.x.is_finite()
        && position.y.is_finite()
        && max_width.is_none_or(|max_width| max_width.is_finite() && max_width > 0.0)
}

pub(in crate::gui_runtime::native_vello) fn font_size_is_renderable(font_size: f32) -> bool {
    font_size.is_finite() && font_size > 0.0
}

#[cfg(test)]
mod tests {
    use super::{text_run_is_renderable, text_run_parts_are_renderable};
    use crate::{
        gui::{
            paint::TextAlign,
            types::{Point, Rgba8},
        },
        gui_runtime::native_vello::text_renderer::SceneTextRun,
    };

    #[test]
    fn text_run_renderability_rejects_non_finite_geometry() {
        let mut position = Point::new(4.0, 8.0);
        let mut font_size = 12.0;
        let mut max_width = Some(80.0);

        assert!(text_run_parts_are_renderable(
            "tempo", position, font_size, max_width
        ));

        font_size = f32::NAN;
        assert!(!text_run_parts_are_renderable(
            "tempo", position, font_size, max_width
        ));

        font_size = 12.0;
        position.x = f32::INFINITY;
        assert!(!text_run_parts_are_renderable(
            "tempo", position, font_size, max_width
        ));

        position.x = 4.0;
        max_width = Some(0.0);
        assert!(!text_run_parts_are_renderable(
            "tempo", position, font_size, max_width
        ));

        max_width = Some(f32::NAN);
        assert!(!text_run_parts_are_renderable(
            "tempo", position, font_size, max_width
        ));
    }

    #[test]
    fn text_run_renderability_accepts_staged_scene_text_runs() {
        let run = SceneTextRun {
            text: "tempo".into(),
            position: Point::new(4.0, 8.0),
            font_size: 12.0,
            color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            max_width: Some(80.0),
            align: TextAlign::Left,
        };

        assert!(text_run_is_renderable(&run));
    }
}
