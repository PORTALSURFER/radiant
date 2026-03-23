use super::*;

#[test]
fn cursor_move_tracks_waveform_hover_position_inside_plot() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.25),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );

    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::None
    );
    let fingerprint = state.state_overlay_fingerprint();
    assert_eq!(fingerprint.hovered, Some(ShellNodeKind::WaveformCard));
    assert!(
        state
            .motion_overlay_fingerprint()
            .waveform_hover_x_bits
            .is_some()
    );
}

#[test]
fn cursor_move_effect_classifies_waveform_hover_only_updates() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let first = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.2),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    let second = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.7),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );

    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, first),
        CursorMoveEffect::GeneralOverlay
    );
    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, second),
        CursorMoveEffect::WaveformHoverOnly
    );
}

#[test]
fn cursor_move_effect_classifies_waveform_toolbar_hover_changes_as_general_overlay() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let channel_rect = state
        .waveform_toolbar_button_rect(&layout, &model, "Channel")
        .expect("channel button should be present");
    let channel = Point::new(
        (channel_rect.min.x + channel_rect.max.x) * 0.5,
        (channel_rect.min.y + channel_rect.max.y) * 0.5,
    );

    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, channel),
        CursorMoveEffect::GeneralOverlay
    );
}

#[test]
fn waveform_toolbar_channel_button_toggles_channel_view_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    let mono_buttons = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    let mono_button = mono_buttons
        .iter()
        .find(|button| button.label == "Channel")
        .expect("channel toolbar button should be present");
    assert_eq!(
        mono_button.action,
        Some(UiAction::SetWaveformChannelView { stereo: true })
    );
    assert_eq!(mono_button.icon, Some(WaveformToolbarIcon::Mono));

    model.waveform_chrome.channel_view = crate::app::WaveformChannelViewModel::Stereo;
    let stereo_buttons = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    let stereo_button = stereo_buttons
        .iter()
        .find(|button| button.label == "Channel")
        .expect("channel toolbar button should be present");
    assert_eq!(
        stereo_button.action,
        Some(UiAction::SetWaveformChannelView { stereo: false })
    );
    assert_eq!(stereo_button.icon, Some(WaveformToolbarIcon::Stereo));
}

#[test]
fn state_overlay_renders_waveform_toolbar_hover_tooltip_text() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let channel_rect = state
        .waveform_toolbar_button_rect(&layout, &model, "Channel")
        .expect("channel button should be present");
    let channel = Point::new(
        (channel_rect.min.x + channel_rect.max.x) * 0.5,
        (channel_rect.min.y + channel_rect.max.y) * 0.5,
    );
    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, channel),
        CursorMoveEffect::GeneralOverlay
    );

    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("Switch waveform view to split stereo"))
    );
}

#[test]
fn cursor_move_clears_waveform_hover_position_outside_plot() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let in_plot = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.4),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    let outside = Point::new(
        layout.sidebar.min.x + 6.0,
        layout.sidebar.min.y + (layout.sidebar.height() * 0.5),
    );

    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, in_plot),
        CursorMoveEffect::None
    );
    assert!(
        state
            .motion_overlay_fingerprint()
            .waveform_hover_x_bits
            .is_some()
    );
    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, outside),
        CursorMoveEffect::None
    );
    assert!(
        state
            .motion_overlay_fingerprint()
            .waveform_hover_x_bits
            .is_none()
    );
}

#[test]
fn waveform_hover_overlay_draws_preview_cursor_marker() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.6),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::None
    );
    let motion = NativeMotionModel::from_app_model(&model);
    let hover_x = f32::from_bits(
        state
            .motion_overlay_fingerprint()
            .waveform_hover_x_bits
            .expect("waveform hover should be tracked"),
    );
    let hover_marker_width = (style.sizing.border_width * 2.0).max(2.0);
    let expected_marker =
        waveform_hover_marker_rect(layout.waveform_plot, hover_marker_width, hover_x)
            .expect("cursor marker rect should exist");
    let expected_color = blend_color(style.accent_warning, style.text_primary, 0.72);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let overlay_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == expected_marker => Some(rect.color),
            _ => None,
        })
        .expect("waveform hover marker should emit a cursor fill rectangle");
    assert_eq!(overlay_color, expected_color);
}
