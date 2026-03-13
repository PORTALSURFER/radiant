use super::*;

#[test]
fn top_bar_omits_status_indicator_dot() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.transport_running = true;

    let frame = state.build_frame(&layout, &model);

    assert!(
        !frame
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Circle(_))),
        "top-right status dot should not be rendered"
    );

    let style = style_for_layout(&layout);
    let motion = NativeMotionModel::from_app_model(&model);
    let mut overlay = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut overlay);

    assert!(
        !overlay
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Circle(_))),
        "motion overlay should not reintroduce the top-right status dot"
    );
}

#[test]
fn waveform_bpm_grid_lines_render_only_when_snap_enabled_and_align_to_beats() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.view_start_milli = 125;
    model.waveform.view_end_milli = 875;
    model.waveform.beat_step_micros = Some(125_000);
    model.waveform_chrome.bpm_snap_enabled = false;
    let beat_line_color = blend_color(style.grid_soft, style.text_muted, 0.32);

    let frame_without_snap = state.build_frame(&layout, &model);
    assert!(
        !frame_without_snap.primitives.iter().any(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(rect)
                    if rect.rect.min.y == layout.waveform_plot.min.y
                        && rect.rect.max.y == layout.waveform_plot.max.y
                        && (rect.color == beat_line_color || rect.color == style.grid_strong)
            )
        }),
        "waveform beat grid should stay hidden when BPM snap is disabled"
    );

    model.waveform_chrome.bpm_snap_enabled = true;
    let frame_with_snap = state.build_frame(&layout, &model);
    let expected_soft_xs = [0.125_f32, 0.25, 0.375, 0.625, 0.75, 0.875]
        .into_iter()
        .map(|beat| beat_grid_x(layout.waveform_plot, 0.125, 0.875, beat))
        .collect::<Vec<_>>();
    let expected_strong_xs = vec![beat_grid_x(layout.waveform_plot, 0.125, 0.875, 0.5)];
    let (actual_soft_xs, actual_strong_xs): (Vec<_>, Vec<_>) = frame_with_snap
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect)
                if rect.rect.min.y == layout.waveform_plot.min.y
                    && rect.rect.max.y == layout.waveform_plot.max.y
                    && (rect.color == beat_line_color || rect.color == style.grid_strong) =>
            {
                Some((rect.color, rect.rect.min.x))
            }
            _ => None,
        })
        .partition(|(color, _)| *color == beat_line_color);
    let actual_soft_xs = actual_soft_xs
        .into_iter()
        .map(|(_, x)| x)
        .collect::<Vec<_>>();
    let actual_strong_xs = actual_strong_xs
        .into_iter()
        .map(|(_, x)| x)
        .collect::<Vec<_>>();

    assert_eq!(actual_soft_xs, expected_soft_xs);
    assert_eq!(actual_strong_xs, expected_strong_xs);
}

fn beat_grid_x(waveform_plot: Rect, view_start: f32, view_end: f32, beat: f32) -> f32 {
    let ratio = (beat - view_start) / (view_end - view_start);
    (waveform_plot.min.x + (waveform_plot.width() * ratio)).round()
}

#[test]
fn waveform_title_uses_primary_text_hierarchy_color() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.loaded_label = Some(String::from("WaveTitle"));
    let frame = state.build_frame(&layout, &model);
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text == "WaveTitle" && run.color == style.text_primary)
    );
}

#[test]
fn waveform_image_data_emits_textured_waveform_primitive() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.waveform_image = Some(std::sync::Arc::new(
        ImageRgba::new(1, 1, vec![11, 22, 33, 255]).unwrap(),
    ));
    let frame = state.build_frame(&layout, &model);
    let has_waveform_image = frame
        .primitives
        .iter()
        .any(|primitive| matches!(primitive, Primitive::Image(image) if image.rect == layout.waveform_plot));
    assert!(has_waveform_image);
}

#[test]
fn waveform_image_data_preserves_distinct_colors_in_texture_payload() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.waveform_image = Some(std::sync::Arc::new(
        ImageRgba::new(
            1,
            2,
            vec![
                11, 22, 33, 255, // top pixel
                99, 88, 77, 255, // bottom pixel
            ],
        )
        .unwrap(),
    ));
    let frame = state.build_frame(&layout, &model);
    let (top_color_present, bottom_color_present) = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Image(image) if image.rect == layout.waveform_plot => Some((
                image.image.pixels.get(0..4) == Some(&[11, 22, 33, 255]),
                image.image.pixels.get(4..8) == Some(&[99, 88, 77, 255]),
            )),
            _ => None,
        })
        .unwrap_or((false, false));
    assert!(top_color_present);
    assert!(bottom_color_present);
}

#[test]
fn waveform_image_transparent_pixels_do_not_emit_texture_primitive() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.waveform_image = Some(std::sync::Arc::new(
        ImageRgba::new(1, 1, vec![11, 22, 33, 0]).unwrap(),
    ));
    let frame = state.build_frame(&layout, &model);
    let has_waveform_image = frame
        .primitives
        .iter()
        .any(|primitive| matches!(primitive, Primitive::Image(image) if image.rect == layout.waveform_plot));
    assert!(!has_waveform_image);
}

#[test]
fn waveform_loading_motion_overlay_covers_plot_with_placeholder() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.loading = true;
    let motion = NativeMotionModel::from_app_model(&model);
    let mut overlay = NativeViewFrame::default();

    state.build_motion_overlay_into(&layout, &style, &motion, &mut overlay);

    assert!(overlay.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect == layout.waveform_plot && rect.color == style.surface_base
        )
    }));
}

#[test]
fn map_header_prefers_projected_legend_selection_and_viewport_copy() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.map.active = true;
    model.map.legend_label = String::from("Render: points");
    model.map.selection_label = String::from("Selection: kick_24.wav");
    model.map.hover_label = String::from("Hover: kick_hover.wav");
    model.map.cluster_label = String::from("Clusters: 7");
    model.map.viewport_label = String::from("zoom 1.75x | pan (12, -8)");
    model.map.summary = String::from("248 points");

    let frame = state.build_frame(&layout, &model);
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("Render: points"))
    );
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("Selection: kick_24.wav"))
    );
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("Clusters: 7"))
    );
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("zoom 1.75x | pan (12, -8)"))
    );
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("248 points"))
    );
}

#[test]
fn map_header_metadata_stays_within_header_band() {
    let layout = ShellLayout::build(Vector2::new(820.0, 520.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.map.active = true;
    model.map.legend_label = String::from("Render: points");
    model.map.selection_label = String::from("Selection: very_long_sample_name.wav");
    model.map.cluster_label = String::from("Clusters: 42");

    let frame = state.build_frame(&layout, &model);
    let header_runs = frame
        .text_runs
        .iter()
        .filter(|run| run.text.contains("Render:") || run.text.contains("Selection:"))
        .collect::<Vec<_>>();
    assert!(!header_runs.is_empty());
    for run in header_runs {
        assert_text_run_inside_band(run, layout.browser_table_header);
    }
}
