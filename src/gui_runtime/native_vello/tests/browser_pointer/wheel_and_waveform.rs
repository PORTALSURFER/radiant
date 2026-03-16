use super::*;

#[test]
fn browser_wheel_delta_is_bounded_and_directional() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(layout.root.rect.width());
    let mut model = AppModel::default();
    model.map.active = false;
    let point = Point::new(
        layout.browser_rows.min.x + 10.0,
        layout.browser_rows.min.y + 10.0,
    );

    assert_eq!(
        browser_wheel_row_delta(
            &layout,
            &model,
            point,
            &style,
            MouseScrollDelta::LineDelta(0.0, 3.0),
        ),
        Some(-3)
    );
    assert_eq!(
        browser_wheel_row_delta(
            &layout,
            &model,
            point,
            &style,
            MouseScrollDelta::LineDelta(0.0, 0.0)
        ),
        None
    );
    let header_point = Point::new(
        layout.browser_table_header.min.x + 5.0,
        layout.browser_table_header.min.y + 5.0,
    );
    assert_eq!(
        browser_wheel_row_delta(
            &layout,
            &model,
            header_point,
            &style,
            MouseScrollDelta::LineDelta(0.0, 2.0),
        ),
        Some(-2)
    );
}

#[test]
fn browser_view_start_after_wheel_clamps_to_visible_bounds() {
    assert_eq!(browser_view_start_after_wheel(10, 40, 12, -3), Some(7));
    assert_eq!(browser_view_start_after_wheel(0, 40, 12, -3), Some(0));
    assert_eq!(browser_view_start_after_wheel(27, 40, 12, 5), Some(28));
    assert_eq!(browser_view_start_after_wheel(4, 0, 12, 2), None);
    assert_eq!(browser_view_start_after_wheel(4, 20, 0, 2), None);
    assert_eq!(browser_view_start_after_wheel(4, 20, 12, 0), None);
}

#[test]
fn waveform_wheel_zoom_requires_waveform_hover_and_maps_direction() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.5),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    assert_eq!(
        waveform_wheel_zoom_action(
            &layout,
            &model,
            point,
            MouseScrollDelta::LineDelta(0.0, 2.0)
        ),
        Some(UiAction::ZoomWaveform {
            zoom_in: true,
            steps: 2,
            anchor_ratio_micros: Some(500_000),
        })
    );
    assert_eq!(
        waveform_wheel_zoom_action(
            &layout,
            &model,
            point,
            MouseScrollDelta::LineDelta(0.0, -1.0)
        ),
        Some(UiAction::ZoomWaveform {
            zoom_in: false,
            steps: 1,
            anchor_ratio_micros: Some(500_000),
        })
    );

    let outside_point = Point::new(
        layout.browser_rows.min.x + 12.0,
        layout.browser_rows.min.y + 12.0,
    );
    assert_eq!(
        waveform_wheel_zoom_action(
            &layout,
            &model,
            outside_point,
            MouseScrollDelta::LineDelta(0.0, 2.0)
        ),
        None
    );
}

#[test]
fn waveform_pointer_position_tracks_active_view_window() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = AppModel::default();
    model.waveform.view_start_micros = 250_000;
    model.waveform.view_end_micros = 750_000;
    let y = layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5);

    let left = Point::new(layout.waveform_plot.min.x, y);
    let center = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.5),
        y,
    );
    let right = Point::new(layout.waveform_plot.max.x, y);

    assert_eq!(
        waveform_position_milli_from_point(&layout, &model, left),
        250
    );
    assert_eq!(
        waveform_position_milli_from_point(&layout, &model, center),
        500
    );
    assert_eq!(
        waveform_position_milli_from_point(&layout, &model, right),
        750
    );
}

#[test]
fn waveform_wheel_zoom_action_uses_pointer_anchor_ratio() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.25),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );

    assert_eq!(
        waveform_wheel_zoom_action(
            &layout,
            &model,
            point,
            MouseScrollDelta::LineDelta(0.0, 1.0)
        ),
        Some(UiAction::ZoomWaveform {
            zoom_in: true,
            steps: 1,
            anchor_ratio_micros: Some(250_000),
        })
    );
}
