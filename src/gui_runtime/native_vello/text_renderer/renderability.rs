use super::SceneTextRun;

pub(super) fn text_run_is_renderable(run: &SceneTextRun) -> bool {
    !run.text.is_empty()
        && font_size_is_renderable(run.font_size)
        && run.position.x.is_finite()
        && run.position.y.is_finite()
        && run
            .max_width
            .is_none_or(|max_width| max_width.is_finite() && max_width > 0.0)
}

pub(in crate::gui_runtime::native_vello) fn font_size_is_renderable(font_size: f32) -> bool {
    font_size.is_finite() && font_size > 0.0
}

#[cfg(test)]
mod tests {
    use super::text_run_is_renderable;
    use crate::{
        gui::{
            paint::TextAlign,
            types::{Point, Rgba8},
        },
        gui_runtime::native_vello::text_renderer::SceneTextRun,
    };

    #[test]
    fn text_run_renderability_rejects_non_finite_geometry() {
        let mut run = SceneTextRun {
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

        run.font_size = f32::NAN;
        assert!(!text_run_is_renderable(&run));

        run.font_size = 12.0;
        run.position.x = f32::INFINITY;
        assert!(!text_run_is_renderable(&run));

        run.position.x = 4.0;
        run.max_width = Some(0.0);
        assert!(!text_run_is_renderable(&run));

        run.max_width = Some(f32::NAN);
        assert!(!text_run_is_renderable(&run));
    }
}
