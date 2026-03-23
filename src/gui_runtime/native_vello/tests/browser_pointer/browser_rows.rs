use super::*;

#[test]
fn browser_row_click_modifiers_route_expected_actions() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = AppModel {
        browser: crate::app::BrowserPanelModel {
            rows: vec![crate::app::BrowserRowModel::new(
                17, "kick-row", 0, false, false,
            )],
            visible_count: 1,
            ..crate::app::BrowserPanelModel::default()
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
fn browser_row_click_targets_interior_row_after_downward_autoscroll() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut shell_state = NativeShellState::new();
    let mut model = AppModel::default();
    for visible_row in 0..40 {
        model.browser.rows.push(crate::app::BrowserRowModel::new(
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
