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
fn waveform_bpm_grid_lines_render_from_sample_origin_when_no_selection_exists() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.view_start_milli = 125;
    model.waveform.view_end_milli = 875;
    model.waveform.beat_step_micros = Some(125_000);
    model.waveform_chrome.bpm_snap_enabled = true;
    let beat_line_color = blend_color(style.grid_soft, style.text_muted, 0.32);

    let frame = state.build_frame(&layout, &model);
    let (soft_xs, strong_xs) = waveform_bpm_grid_positions(&frame, &layout, &style);

    let expected_soft_xs = [0.125_f32, 0.25, 0.375, 0.625, 0.75, 0.875]
        .into_iter()
        .map(|beat| beat_grid_x(layout.waveform_plot, 0.125, 0.875, beat))
        .collect::<Vec<_>>();
    let expected_strong_xs = vec![beat_grid_x(layout.waveform_plot, 0.125, 0.875, 0.5)];

    assert_eq!(soft_xs, expected_soft_xs);
    assert_eq!(strong_xs, expected_strong_xs);

    assert!(
        frame.primitives.iter().any(|primitive| matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.y == layout.waveform_plot.min.y
                    && rect.rect.max.y == layout.waveform_plot.max.y
                    && rect.color == beat_line_color
        )),
        "waveform beat grid should render when BPM snap is enabled"
    );
}

#[test]
fn waveform_bpm_grid_lines_reuse_last_selection_origin_after_clear() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.view_start_milli = 125;
    model.waveform.view_end_milli = 875;
    model.waveform.beat_step_micros = Some(125_000);
    model.waveform_chrome.bpm_snap_enabled = true;
    model.waveform.selection_milli = Some(crate::app::NormalizedRangeModel::new(125, 375));

    let selected_frame = state.build_frame(&layout, &model);
    let selected_positions = waveform_bpm_grid_positions(&selected_frame, &layout, &style);
    let expected_selected_soft_xs = [0.25_f32, 0.375, 0.5, 0.75, 0.875]
        .into_iter()
        .map(|beat| beat_grid_x(layout.waveform_plot, 0.125, 0.875, beat))
        .collect::<Vec<_>>();
    let expected_selected_strong_xs = vec![
        beat_grid_x(layout.waveform_plot, 0.125, 0.875, 0.125),
        beat_grid_x(layout.waveform_plot, 0.125, 0.875, 0.625),
    ];

    assert_eq!(selected_positions.0, expected_selected_soft_xs);
    assert_eq!(selected_positions.1, expected_selected_strong_xs);

    model.waveform.selection_milli = None;
    let cleared_frame = state.build_frame(&layout, &model);
    let cleared_positions = waveform_bpm_grid_positions(&cleared_frame, &layout, &style);
    let expected_cleared_soft_xs = [0.25_f32, 0.375, 0.5, 0.75, 0.875]
        .into_iter()
        .map(|beat| beat_grid_x(layout.waveform_plot, 0.125, 0.875, beat))
        .collect::<Vec<_>>();
    let expected_cleared_strong_xs = vec![
        beat_grid_x(layout.waveform_plot, 0.125, 0.875, 0.125),
        beat_grid_x(layout.waveform_plot, 0.125, 0.875, 0.625),
    ];

    assert_eq!(cleared_positions.0, expected_cleared_soft_xs);
    assert_eq!(cleared_positions.1, expected_cleared_strong_xs);
}

#[test]
fn waveform_bpm_grid_lines_prefer_projected_origin_when_present() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.view_start_milli = 125;
    model.waveform.view_end_milli = 875;
    model.waveform.beat_step_micros = Some(125_000);
    model.waveform.bpm_grid_origin_micros = 250_000;
    model.waveform_chrome.bpm_snap_enabled = true;

    let frame = state.build_frame(&layout, &model);
    let (soft_xs, strong_xs) = waveform_bpm_grid_positions(&frame, &layout, &style);

    let expected_soft_xs = [0.375_f32, 0.5, 0.625, 0.875]
        .into_iter()
        .map(|beat| beat_grid_x(layout.waveform_plot, 0.125, 0.875, beat))
        .collect::<Vec<_>>();
    let expected_strong_xs = vec![
        beat_grid_x(layout.waveform_plot, 0.125, 0.875, 0.25),
        beat_grid_x(layout.waveform_plot, 0.125, 0.875, 0.75),
    ];

    assert_eq!(soft_xs, expected_soft_xs);
    assert_eq!(strong_xs, expected_strong_xs);
}

fn beat_grid_x(waveform_plot: Rect, view_start: f32, view_end: f32, beat: f32) -> f32 {
    let ratio = (beat - view_start) / (view_end - view_start);
    (waveform_plot.min.x + (waveform_plot.width() * ratio)).round()
}

fn waveform_bpm_grid_positions(
    frame: &NativeViewFrame,
    layout: &ShellLayout,
    style: &StyleTokens,
) -> (Vec<f32>, Vec<f32>) {
    let beat_line_color = blend_color(style.grid_soft, style.text_muted, 0.32);
    let mut soft_xs = Vec::new();
    let mut strong_xs = Vec::new();
    for primitive in &frame.primitives {
        let Primitive::Rect(rect) = primitive else {
            continue;
        };
        if rect.rect.min.y != layout.waveform_plot.min.y
            || rect.rect.max.y != layout.waveform_plot.max.y
        {
            continue;
        }
        if rect.color == beat_line_color {
            soft_xs.push(rect.rect.min.x);
        } else if rect.color == style.grid_strong {
            strong_xs.push(rect.rect.min.x);
        }
    }
    (soft_xs, strong_xs)
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
