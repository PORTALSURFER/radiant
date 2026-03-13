use super::*;

#[test]
fn source_header_add_button_click_maps_to_add_source_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = populated_sidebar_model();
    let mut state = NativeShellState::new();
    let button = state
        .source_add_button_rect(&layout)
        .expect("source add button should render");
    assert_rect_inside(layout.sidebar_header, button);
    let point = Point::new(
        button.min.x + (button.width() * 0.5),
        button.min.y + (button.height() * 0.5),
    );
    let action = state
        .source_action_at_point(&layout, &model, point)
        .expect("source add button click should produce action");
    assert_eq!(action, UiAction::OpenAddSourceDialog);
}

#[test]
fn source_header_add_button_hover_sets_motion_overlay_fingerprint() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = populated_sidebar_model();
    let mut state = NativeShellState::new();
    let button = state
        .source_add_button_rect(&layout)
        .expect("source add button should render");
    let point = Point::new(
        (button.min.x + button.max.x) * 0.5,
        (button.min.y + button.max.y) * 0.5,
    );

    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::GeneralOverlay
    );

    let fingerprint = state.chrome_motion_overlay_fingerprint();
    assert!(fingerprint.hovered_source_add_button);
    assert!(!fingerprint.flashed_source_add_button);
    assert_eq!(fingerprint.source_add_button_flash_ticks, 0);
}

#[test]
fn browser_search_field_hover_sets_motion_overlay_fingerprint() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let style = style_for_layout(&layout);
    let (_, _, toolbar) = state.cached_browser_action_hit_test(&layout, &style, &model);
    let point = Point::new(
        (toolbar.search_field.min.x + toolbar.search_field.max.x) * 0.5,
        (toolbar.search_field.min.y + toolbar.search_field.max.y) * 0.5,
    );

    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::GeneralOverlay
    );

    let fingerprint = state.chrome_motion_overlay_fingerprint();
    assert!(fingerprint.hovered_browser_search_field);
}

#[test]
fn browser_search_field_motion_overlay_uses_hover_fill() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let model = populated_sidebar_model();
    let motion = NativeMotionModel::from_app_model(&model);
    let mut state = NativeShellState::new();
    let (_, _, toolbar) = state.cached_browser_action_hit_test(&layout, &style, &model);
    let point = Point::new(
        (toolbar.search_field.min.x + toolbar.search_field.max.x) * 0.5,
        (toolbar.search_field.min.y + toolbar.search_field.max.y) * 0.5,
    );

    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::GeneralOverlay
    );

    let mut frame = NativeViewFrame::default();
    state.build_chrome_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let overlay_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(FillRect { rect, color }) if *rect == toolbar.search_field => {
                Some(*color)
            }
            _ => None,
        })
        .expect("hovered browser search field should emit a motion overlay fill");

    assert_eq!(
        overlay_color,
        browser_search_field_hover_fill(&style, interaction_wave(0.0))
    );
}

#[test]
fn browser_rating_filter_chip_hover_sets_motion_overlay_fingerprint() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let chip = state
        .browser_rating_filter_chip_rect(&layout, &model, 3)
        .expect("keep-3 chip should render");
    let point = Point::new(
        (chip.min.x + chip.max.x) * 0.5,
        (chip.min.y + chip.max.y) * 0.5,
    );

    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::GeneralOverlay
    );

    let fingerprint = state.chrome_motion_overlay_fingerprint();
    assert_eq!(fingerprint.hovered_browser_rating_filter_level, Some(3));
}

#[test]
fn browser_rating_filter_chip_motion_overlay_uses_hover_fill() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let model = AppModel::default();
    let motion = NativeMotionModel::from_app_model(&model);
    let mut state = NativeShellState::new();
    let chip = state
        .browser_rating_filter_chip_rect(&layout, &model, 3)
        .expect("keep-3 chip should render");
    let point = Point::new(
        (chip.min.x + chip.max.x) * 0.5,
        (chip.min.y + chip.max.y) * 0.5,
    );

    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::GeneralOverlay
    );

    let mut frame = NativeViewFrame::default();
    state.build_chrome_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let overlay_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(FillRect { rect, color }) if *rect == chip => Some(*color),
            _ => None,
        })
        .expect("hovered browser rating chip should emit a motion overlay fill");

    assert_eq!(
        overlay_color,
        browser_rating_filter_chip_hover_fill(&style, 3, false, interaction_wave(0.0))
    );
}

#[test]
fn browser_search_state_overlay_renders_active_editor_selection_and_caret() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let search_field = state
        .browser_search_field_rect(&layout, &model)
        .expect("search field should render");
    let search_text = state
        .browser_search_text_rect(&layout, &model)
        .expect("search text rect should render");
    state.set_browser_search_editor_state(Some(TextFieldVisualState {
        text: String::from("kick"),
        caret_offset: 18.0,
        selection_offsets: Some((0.0, 12.0)),
    }));

    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);
    let caret_width = style.sizing.border_width.max(1.0);

    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == search_field && *color == browser_search_field_active_fill(&style)
        )
    }));
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, .. })
                if *rect
                    == Rect::from_min_max(
                        Point::new(search_text.min.x, search_text.min.y),
                        Point::new(search_text.min.x + 12.0, search_text.max.y),
                    )
        )
    }));
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, .. })
                if *rect
                    == Rect::from_min_max(
                        Point::new(search_text.min.x + 18.0, search_text.min.y),
                        Point::new(search_text.min.x + 18.0 + caret_width, search_text.max.y),
                    )
        )
    }));
    assert!(frame.text_runs.iter().any(|run| run.text == "kick"));
}

#[test]
fn browser_rating_filter_chip_uses_active_fill_when_enabled() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut model = AppModel::default();
    model.browser.active_rating_filters[6] = true;
    let mut state = NativeShellState::new();
    let chip = state
        .browser_rating_filter_chip_rect(&layout, &model, 3)
        .expect("keep-3 chip should render");

    let frame = state.build_frame(&layout, &model);
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == chip && *color == browser_rating_filter_chip_fill(&style, 3, true)
        )
    }));
}

#[test]
fn locked_browser_rating_filter_chip_uses_active_fill_when_enabled() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut model = AppModel::default();
    model.browser.active_rating_filters[7] = true;
    let mut state = NativeShellState::new();
    let chip = state
        .browser_rating_filter_chip_rect(&layout, &model, 4)
        .expect("locked keep chip should render");

    let frame = state.build_frame(&layout, &model);
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == chip && *color == browser_rating_filter_chip_fill(&style, 4, true)
        )
    }));
}

#[test]
fn browser_rating_filter_chip_hover_preserves_active_fill_when_enabled() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut model = AppModel::default();
    model.browser.active_rating_filters[6] = true;
    let motion = NativeMotionModel::from_app_model(&model);
    let mut state = NativeShellState::new();
    let chip = state
        .browser_rating_filter_chip_rect(&layout, &model, 3)
        .expect("keep-3 chip should render");
    let point = Point::new(
        (chip.min.x + chip.max.x) * 0.5,
        (chip.min.y + chip.max.y) * 0.5,
    );

    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::GeneralOverlay
    );

    let mut frame = NativeViewFrame::default();
    state.build_chrome_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let overlay_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(FillRect { rect, color }) if *rect == chip => Some(*color),
            _ => None,
        })
        .expect("hovered active browser rating chip should emit a motion overlay fill");

    assert_eq!(
        overlay_color,
        browser_rating_filter_chip_hover_fill(&style, 3, true, interaction_wave(0.0))
    );
}

#[test]
fn source_header_add_button_click_sets_flash_in_chrome_motion_fingerprint() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = populated_sidebar_model();
    let mut state = NativeShellState::new();
    let button = state
        .source_add_button_rect(&layout)
        .expect("source add button should render");
    let point = Point::new(
        (button.min.x + button.max.x) * 0.5,
        (button.min.y + button.max.y) * 0.5,
    );

    assert_eq!(
        state.source_action_at_point(&layout, &model, point),
        Some(UiAction::OpenAddSourceDialog)
    );

    let fingerprint = state.chrome_motion_overlay_fingerprint();
    assert!(fingerprint.flashed_source_add_button);
    assert!(fingerprint.source_add_button_flash_ticks > 0);
}

#[test]
fn source_header_add_button_motion_overlay_uses_hover_fill() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let model = populated_sidebar_model();
    let motion = NativeMotionModel::from_app_model(&model);
    let mut state = NativeShellState::new();
    let button = state
        .source_add_button_rect(&layout)
        .expect("source add button should render");
    let point = Point::new(
        (button.min.x + button.max.x) * 0.5,
        (button.min.y + button.max.y) * 0.5,
    );

    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::GeneralOverlay
    );

    let mut frame = NativeViewFrame::default();
    state.build_chrome_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let expected_fill = source_add_button_fill(&style, true, false, interaction_wave(0.0));
    let overlay_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(FillRect { rect, color }) if *rect == button => Some(*color),
            _ => None,
        })
        .expect("hovered source add button should emit a motion overlay fill");

    assert_eq!(overlay_color, expected_fill);
}

#[test]
fn browser_rating_indicator_layout_stays_inside_sample_label() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let sizing = style.sizing;
    let row_rect = Rect::from_min_max(Point::new(80.0, 120.0), Point::new(520.0, 156.0));
    let row_text = compute_browser_row_text_layout(row_rect, sizing);
    let keep = browser_rating_indicator_layout(
        BrowserRatingIndicatorAnchor {
            sample_label: row_text.sample_label,
            label_origin_x: row_text.sample_label.min.x,
            label_rendered_width: 42.0,
            right_limit_x: row_text.sample_label.max.x,
        },
        3,
        false,
        sizing,
    )
    .expect("keep indicators should render");
    let trash = browser_rating_indicator_layout(
        BrowserRatingIndicatorAnchor {
            sample_label: row_text.sample_label,
            label_origin_x: row_text.sample_label.min.x,
            label_rendered_width: 42.0,
            right_limit_x: row_text.sample_label.max.x,
        },
        -2,
        false,
        sizing,
    )
    .expect("trash indicators should render");
    assert_eq!(keep.count, 3);
    assert_eq!(trash.count, 2);
    for rect in keep.rects.iter().take(keep.count) {
        assert_rect_inside(row_text.sample_label, *rect);
    }
    for rect in trash.rects.iter().take(trash.count) {
        assert_rect_inside(row_text.sample_label, *rect);
    }
    assert_eq!(browser_rating_indicator_color(&style, 3), style.accent_mint);
    assert_eq!(
        browser_rating_indicator_color(&style, -2),
        style.accent_trash
    );
}

#[test]
fn browser_rating_indicator_layout_trails_rendered_label() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let sizing = style.sizing;
    let row_rect = Rect::from_min_max(Point::new(80.0, 120.0), Point::new(520.0, 156.0));
    let row_text = compute_browser_row_text_layout(row_rect, sizing);
    let label_origin_x = row_text.sample_label.min.x + 18.0;
    let label_rendered_width = 64.0;
    let right_limit_x = row_text.sample_label.max.x - 48.0;
    let indicators = browser_rating_indicator_layout(
        BrowserRatingIndicatorAnchor {
            sample_label: row_text.sample_label,
            label_origin_x,
            label_rendered_width,
            right_limit_x,
        },
        2,
        false,
        sizing,
    )
    .expect("rating indicators should render");
    let expected_min_x =
        label_origin_x + label_rendered_width + browser_rating_indicator_text_gap(sizing);
    let first_rect = indicators.rects[0];
    let last_rect = indicators.rects[indicators.count - 1];
    assert!(first_rect.min.x >= expected_min_x);
    assert!(last_rect.max.x <= right_limit_x);
}

#[test]
fn locked_keep_rating_indicator_uses_single_wide_rect() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let sizing = style.sizing;
    let row_rect = Rect::from_min_max(Point::new(80.0, 120.0), Point::new(520.0, 156.0));
    let row_text = compute_browser_row_text_layout(row_rect, sizing);
    let indicators = browser_rating_indicator_layout(
        BrowserRatingIndicatorAnchor {
            sample_label: row_text.sample_label,
            label_origin_x: row_text.sample_label.min.x,
            label_rendered_width: 42.0,
            right_limit_x: row_text.sample_label.max.x,
        },
        3,
        true,
        sizing,
    )
    .expect("locked keep indicator should render");

    assert_eq!(indicators.count, 1);
    assert!(indicators.rects[0].width() > indicators.rects[0].height());
    assert_eq!(
        browser_rating_indicator_reserved_width(3, true, sizing),
        indicators.rects[0].width() + browser_rating_indicator_text_gap(sizing)
    );
}

#[test]
fn browser_action_buttons_stay_inside_toolbar() {
    let mut model = AppModel::default();
    model.browser_actions.can_rename = true;
    model.browser_actions.can_tag = true;
    model.browser_actions.can_delete = true;
    for viewport in [
        Vector2::new(820.0, 520.0),
        Vector2::new(1280.0, 720.0),
        Vector2::new(2300.0, 1080.0),
    ] {
        let layout = ShellLayout::build(viewport);
        let style = style_for_layout(&layout);
        let buttons = browser_action_buttons(&layout, &style, &model);
        assert_eq!(buttons.len(), 1);
        assert_eq!(buttons[0].label, "Random");
        assert!(buttons[0].enabled);
        assert!(!buttons[0].active);
        assert_rect_inside(layout.browser_toolbar, buttons[0].rect);
    }
}

#[test]
fn browser_toolbar_controls_do_not_overlap_action_cluster() {
    let mut model = AppModel::default();
    model.browser_actions.can_rename = true;
    model.browser_actions.can_tag = true;
    model.browser_actions.can_delete = true;
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let buttons = browser_action_buttons(&layout, &style, &model);
    let controls = browser_toolbar_layout(&layout, &style, &buttons);
    assert_eq!(buttons.len(), 1);
    assert!(
        controls
            .rating_filter_chips
            .iter()
            .all(|chip| chip.width() > 1.0)
    );
    assert_rect_inside(layout.browser_toolbar, controls.search_field);
    assert!(controls.search_field.max.x <= buttons[0].rect.min.x);
    assert!(controls.rating_filter_chips[7].max.x <= controls.search_field.min.x);
    assert!(controls.search_field.width() < layout.browser_toolbar.width());
    assert!(controls.activity_chip.width() <= 0.0);
    assert!(controls.sort_chip.width() <= 0.0);
    assert!(
        controls
            .triage_chips
            .into_iter()
            .all(|chip| chip.width() <= 0.0)
    );
}

#[test]
fn browser_toolbar_right_side_does_not_hit_search_field() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let model = AppModel::default();
    let style = style_for_layout(&layout);
    let buttons = browser_action_buttons(&layout, &style, &model);
    let controls = browser_toolbar_layout(&layout, &style, &buttons);
    let point = Point::new(
        (controls.search_field.max.x + layout.browser_toolbar.max.x) * 0.5,
        (layout.browser_toolbar.min.y + layout.browser_toolbar.max.y) * 0.5,
    );
    assert!(point.x > controls.search_field.max.x);
    assert_eq!(
        state.browser_action_at_point(&layout, &model, point, false),
        None
    );
}

#[test]
fn browser_random_action_button_click_maps_to_toggle_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let button = state
        .browser_action_button_rect(&layout, &model, "Random")
        .expect("random button should render");
    let point = Point::new(
        (button.min.x + button.max.x) * 0.5,
        (button.min.y + button.max.y) * 0.5,
    );

    assert_eq!(
        state.browser_action_at_point(&layout, &model, point, false),
        Some(UiAction::ToggleRandomNavigationMode)
    );
}

#[test]
fn top_bar_controls_fit_inside_control_row() {
    for viewport in [
        Vector2::new(820.0, 520.0),
        Vector2::new(1280.0, 720.0),
        Vector2::new(2300.0, 1080.0),
    ] {
        let layout = ShellLayout::build(viewport);
        let style = style_for_layout(&layout);
        let controls = top_bar_controls_layout(&layout, style.sizing);
        if !controls.active {
            continue;
        }
        assert_rect_inside(layout.top_bar_title_cluster, controls.volume_meter);
        assert_rect_inside(layout.top_bar_title_cluster, controls.volume_value);
        assert_rect_inside(layout.top_bar_title_cluster, controls.volume_label);
        assert!(controls.volume_meter.max.x <= controls.volume_value.min.x);
        assert!(controls.volume_value.max.x <= controls.volume_label.min.x);
    }
}
