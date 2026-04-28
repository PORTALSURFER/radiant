use super::*;

#[test]
fn browser_row_click_modifiers_route_expected_actions() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = AppModel {
        browser: crate::sempal_app::BrowserPanelModel {
            rows: vec![crate::sempal_app::BrowserRowModel::new(
                17, "kick-row", 0, false, false,
            )]
            .into(),
            visible_count: 1,
            ..crate::sempal_app::BrowserPanelModel::default()
        },
        ..AppModel::default()
    };
    let row_center_y = layout.browser_rows.min.y
        + (StyleTokens::for_viewport_width(layout.root.rect.width())
            .sizing
            .browser_row_height
            * 0.5);
    let point = Point::new(
        (layout.browser_rows.min.x + layout.browser_rows.max.x) * 0.5,
        row_center_y,
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::FocusBrowserRow { visible_row: 17 })
    );
    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::SHIFT
        ),
        Some(UiAction::ExtendBrowserSelectionToRow { visible_row: 17 })
    );
    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::CONTROL,
        ),
        Some(UiAction::ToggleBrowserRowSelection { visible_row: 17 })
    );
    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::SUPER,
        ),
        Some(UiAction::ToggleBrowserRowSelection { visible_row: 17 })
    );
    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::SHIFT | ModifiersState::SUPER,
        ),
        Some(UiAction::AddRangeBrowserSelection { visible_row: 17 })
    );
}

#[test]
fn focused_browser_row_similarity_button_routes_toggle_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = AppModel {
        browser: crate::sempal_app::BrowserPanelModel {
            rows: vec![crate::sempal_app::BrowserRowModel::new(
                0, "kick-row", 1, true, true,
            )]
            .into(),
            visible_count: 1,
            selected_visible_row: Some(0),
            similarity_filtered: true,
            ..crate::sempal_app::BrowserPanelModel::default()
        },
        ..AppModel::default()
    };
    let button = shell_state
        .browser_similarity_button_rect(&layout, &model)
        .expect("focused row should expose a similarity button");
    let point = Point::new(
        (button.min.x + button.max.x) * 0.5,
        (button.min.y + button.max.y) * 0.5,
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::ToggleFindSimilarFocusedSample)
    );
}

#[test]
fn browser_row_click_targets_interior_row_after_downward_autoscroll() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut shell_state = NativeShellState::new();
    let mut model = AppModel::default();
    for visible_row in 0..40 {
        model
            .browser
            .rows
            .push(crate::sempal_app::BrowserRowModel::new(
                visible_row,
                format!("row_{visible_row:02}"),
                1,
                false,
                visible_row == 18,
            ));
    }
    model.browser.visible_count = model.browser.rows.len();
    model.browser.autoscroll = true;
    model.browser.selected_visible_row = Some(18);
    model.browser.anchor_visible_row = Some(18);
    let row_stride = style.sizing.browser_row_height + style.sizing.browser_row_gap;
    let row_center_y =
        layout.browser_rows.min.y + (row_stride * 11.0) + (style.sizing.browser_row_height * 0.5);
    let point = Point::new(layout.browser_rows.min.x + 24.0, row_center_y);

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::FocusBrowserRow { visible_row: 12 })
    );
}

#[test]
fn browser_row_right_click_routes_duplicate_cleanup_keep_toggle() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.model = Arc::new(AppModel {
        browser: crate::sempal_app::BrowserPanelModel {
            rows: vec![crate::sempal_app::BrowserRowModel::new(
                5, "kick-row", 1, false, true,
            )]
            .into(),
            visible_count: 1,
            duplicate_cleanup_active: true,
            ..crate::sempal_app::BrowserPanelModel::default()
        },
        focus_context: crate::sempal_app::FocusContextModel::SampleBrowser,
        ..AppModel::default()
    });
    runner.frame_state.model_dirty = false;
    let point = (layout.browser_rows.min.x as i32..=layout.browser_rows.max.x as i32)
        .find_map(|x| {
            (layout.browser_rows.min.y as i32..=layout.browser_rows.max.y as i32).find_map(|y| {
                let point = Point::new(x as f32, y as f32);
                (runner
                    .shell_state
                    .browser_row_at_point(&layout, &runner.model, point)
                    == Some(5))
                .then_some(point)
            })
        })
        .expect("duplicate cleanup row should be hittable");
    let mut action_emitted = false;
    let mut source_menu_state_changed = false;

    assert!(runner.handle_right_pointer_press_for_tests(
        &layout,
        point,
        &mut action_emitted,
        &mut source_menu_state_changed,
    ));
    assert!(action_emitted);
    assert!(!source_menu_state_changed);
    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::ToggleBrowserDuplicateCleanupKeep { visible_row: 5 }]
    );
}
