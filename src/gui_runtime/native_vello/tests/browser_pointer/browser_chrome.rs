use super::*;

#[test]
fn browser_toolbar_alt_click_maps_to_inverted_rating_filter_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut shell_state = NativeShellState::new();
    let chip = shell_state
        .content_rating_filter_chip_rect(&layout, &model, 4)
        .expect("locked keep rating filter chip should exist");
    let point = Point::new(
        (chip.min.x + chip.max.x) * 0.5,
        (chip.min.y + chip.max.y) * 0.5,
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::ALT,
        ),
        Some(UiAction::ToggleContentRatingFilter {
            level: 4,
            invert: true,
        })
    );
}

#[test]
fn browser_toolbar_click_maps_to_content_recency_filter_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut shell_state = NativeShellState::new();
    let chip = shell_state
        .content_recency_filter_chip_rect(
            &layout,
            &model,
            crate::compat_app_contract::PlaybackAgeFilterChip::OlderThanMonth,
        )
        .expect("month playback-age filter chip should exist");
    let point = Point::new(
        (chip.min.x + chip.max.x) * 0.5,
        (chip.min.y + chip.max.y) * 0.5,
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::ToggleContentRecencyFilter {
            chip: crate::gui::list::RecencyFilterChip::OlderThanMonth,
            invert: false,
        })
    );
}

#[test]
fn browser_random_action_button_click_routes_toggle_random_navigation_mode() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut shell_state = NativeShellState::new();
    let button = shell_state
        .content_action_button_rect(&layout, &model, "Random")
        .expect("random browser action button should exist");
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
        Some(UiAction::ToggleRandomNavigationMode)
    );
}

#[test]
fn browser_cleanup_action_button_click_routes_toggle_duplicate_cleanup_mode() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut shell_state = NativeShellState::new();
    let button = shell_state
        .content_action_button_rect(&layout, &model, "Cleanup")
        .expect("cleanup browser action button should exist");
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
        Some(UiAction::ToggleContentDuplicateCleanupMode)
    );
}

#[test]
fn browser_tab_clicks_route_to_tab_actions() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = AppModel::default();
    let map_tab_point = Point::new(
        layout.browser_tabs.min.x + (layout.browser_tabs.width() * 0.75),
        layout.browser_tabs.min.y + (layout.browser_tabs.height() * 0.5),
    );
    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            map_tab_point,
            ModifiersState::default(),
        ),
        Some(UiAction::SetContentTab { map: true })
    );

    let list_tab_point = Point::new(
        layout.browser_tabs.min.x + (layout.browser_tabs.width() * 0.25),
        layout.browser_tabs.min.y + (layout.browser_tabs.height() * 0.5),
    );
    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            list_tab_point,
            ModifiersState::default(),
        ),
        Some(UiAction::SetContentTab { map: false })
    );
}
