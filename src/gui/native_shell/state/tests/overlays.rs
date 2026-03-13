use super::*;

#[test]
fn hovered_sections_do_not_emit_panel_fill_overlays() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    for hovered in [
        ShellNodeKind::TopBar,
        ShellNodeKind::Sidebar,
        ShellNodeKind::WaveformCard,
    ] {
        let mut frame = NativeViewFrame::default();
        state.hovered = Some(hovered);
        state.build_state_overlay_into(&layout, &style, &model, &mut frame);
        assert!(
            frame.primitives.iter().all(|primitive| {
                !matches!(
                    primitive,
                    Primitive::Rect(rect)
                        if rect.rect == layout.top_bar
                            || rect.rect == layout.sidebar
                            || rect.rect == layout.waveform_card
                )
            }),
            "hovered section should not emit a panel-sized fill overlay"
        );
    }
}

#[test]
fn browser_row_hovered_overlay_uses_hover_fill() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "hover", 1, false, false));
    model
        .browser
        .rows
        .push(BrowserRowModel::new(1, "hover-2", 1, false, false));
    model.browser.visible_count = model.browser.rows.len();

    let rendered_rows = rendered_browser_rows(&layout, &model, &style);
    let hover_row = rendered_rows[0].rect;
    let cursor = Point::new(
        hover_row.min.x + 4.0,
        (hover_row.min.y + hover_row.max.y) * 0.5,
    );
    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, cursor),
        CursorMoveEffect::None
    );

    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    let expected_hover = browser_row_hover_fill(&style);
    let overlay_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == hover_row => Some(rect.color),
            _ => None,
        })
        .expect("hovered browser row should emit a fill rectangle");

    assert_eq!(overlay_color, expected_hover);
}

#[test]
fn folder_row_hovered_overlay_uses_hover_fill() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let style = StyleTokens::for_viewport_width(1280.0);
    let model = populated_sidebar_model();

    let rendered_rows = rendered_folder_row_rects(&layout, &style, &model);
    let hover_row = rendered_rows[0];
    let cursor = Point::new(
        hover_row.min.x + 4.0,
        (hover_row.min.y + hover_row.max.y) * 0.5,
    );
    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, cursor),
        CursorMoveEffect::None
    );

    let fingerprint = state.state_overlay_fingerprint();
    assert_eq!(fingerprint.hovered, Some(ShellNodeKind::Sidebar));
    assert_eq!(fingerprint.hovered_folder_row_index, Some(0));

    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    let expected_hover = subtle_item_hover_fill(&style);
    let overlay_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == hover_row => Some(rect.color),
            _ => None,
        })
        .expect("hovered folder row should emit a fill rectangle");

    assert_eq!(overlay_color, expected_hover);
}

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

#[test]
fn stale_static_browser_rows_do_not_keep_old_focus_highlight_after_refocus() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let old_model = browser_model_with_rows(40, 18);
    let new_model = browser_model_with_rows(40, 12);
    let focus_border = blend_color(
        style.accent_warning,
        style.text_primary,
        style.state_focus_pulse_blend,
    );

    let old_row_rect = rendered_browser_rows(&layout, &old_model, &style)
        .into_iter()
        .find(|row| row.visible_row == 18)
        .map(|row| row.rect)
        .expect("old focused row should render");
    let new_row_rect = rendered_browser_rows(&layout, &new_model, &style)
        .into_iter()
        .find(|row| row.visible_row == 12)
        .map(|row| row.rect)
        .expect("new focused row should render");

    let mut segments = StaticFrameSegments::default();
    state.build_static_segment_with_style_into(
        &layout,
        &style,
        &old_model,
        None,
        StaticFrameSegment::BrowserRowsWindow,
        &mut segments,
    );

    let static_frame = segments.frame(StaticFrameSegment::BrowserRowsWindow);
    assert!(
        static_frame.primitives.iter().all(|primitive| {
            !matches!(
                primitive,
                Primitive::Rect(rect) if rect.rect.min.y >= old_row_rect.min.y
                    && rect.rect.max.y <= old_row_rect.max.y
                    && rect.color == focus_border
            )
        }),
        "static browser rows should not own focused-row warning chrome"
    );

    state.sync_from_model(&new_model);
    let mut overlay = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &new_model, &mut overlay);

    let old_focus_rects = overlay
        .primitives
        .iter()
        .filter(|primitive| match primitive {
            Primitive::Rect(rect) => {
                rect.rect.min.y >= old_row_rect.min.y
                    && rect.rect.max.y <= old_row_rect.max.y
                    && rect.color == focus_border
            }
            _ => false,
        })
        .count();
    let new_focus_rects = overlay
        .primitives
        .iter()
        .filter(|primitive| match primitive {
            Primitive::Rect(rect) => {
                rect.rect.min.y >= new_row_rect.min.y
                    && rect.rect.max.y <= new_row_rect.max.y
                    && rect.color == focus_border
            }
            _ => false,
        })
        .count();

    assert_eq!(
        old_focus_rects, 0,
        "fresh overlay should not keep the old focused row highlighted"
    );
    assert!(
        new_focus_rects > 0,
        "fresh overlay should highlight the newly focused row"
    );
}

#[test]
fn stale_static_browser_rows_do_not_keep_old_selection_fill_after_refocus() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut old_model = browser_model_with_rows(40, 18);
    let mut new_model = browser_model_with_rows(40, 12);
    if let Some(row) = old_model.browser.rows.get_mut(18) {
        row.selected = true;
    }
    if let Some(row) = new_model.browser.rows.get_mut(12) {
        row.selected = true;
    }
    let old_row_rect = rendered_browser_rows(&layout, &old_model, &style)
        .into_iter()
        .find(|row| row.visible_row == 18)
        .map(|row| row.rect)
        .expect("old selected row should render");
    let selected_fill = selected_browser_row_fill(&style);

    let mut segments = StaticFrameSegments::default();
    state.build_static_segment_with_style_into(
        &layout,
        &style,
        &old_model,
        None,
        StaticFrameSegment::BrowserRowsWindow,
        &mut segments,
    );

    let static_frame = segments.frame(StaticFrameSegment::BrowserRowsWindow);
    assert!(
        static_frame.primitives.iter().all(|primitive| {
            !matches!(
                primitive,
                Primitive::Rect(rect) if rect.rect == old_row_rect && rect.color == selected_fill
            )
        }),
        "static browser rows should not own selected-row fill"
    );

    state.sync_from_model(&new_model);
    let mut overlay = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &new_model, &mut overlay);

    assert!(
        overlay.primitives.iter().all(|primitive| {
            !matches!(
                primitive,
                Primitive::Rect(rect) if rect.rect == old_row_rect && rect.color == selected_fill
            )
        }),
        "fresh overlay should not keep the old selected row filled"
    );
}

#[test]
fn clearing_browser_row_hover_removes_unrelated_hover_fill() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let model = browser_model_with_rows(40, 18);
    let hovered_row = rendered_browser_rows(&layout, &model, &style)
        .into_iter()
        .find(|row| row.visible_row == 12)
        .map(|row| row.rect)
        .expect("hover target row should render");
    let hover_point = Point::new(
        hovered_row.min.x + 6.0,
        (hovered_row.min.y + hovered_row.max.y) * 0.5,
    );

    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, hover_point),
        CursorMoveEffect::None
    );
    assert_eq!(
        state.state_overlay_fingerprint().hovered_browser_visible_row,
        Some(12)
    );

    state.clear_browser_row_hover();
    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    assert_eq!(
        state.state_overlay_fingerprint().hovered_browser_visible_row,
        None
    );
    assert!(
        frame.primitives.iter().all(|primitive| {
            !matches!(primitive, Primitive::Rect(rect) if rect.rect == hovered_row && rect.color == browser_row_hover_fill(&style))
        }),
        "cleared browser row hover should remove the row-hover fill"
    );
}
