use super::*;

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
fn browser_cleanup_action_button_click_maps_to_toggle_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let button = state
        .browser_action_button_rect(&layout, &model, "Cleanup")
        .expect("cleanup button should render");
    let point = Point::new(
        (button.min.x + button.max.x) * 0.5,
        (button.min.y + button.max.y) * 0.5,
    );

    assert_eq!(
        state.browser_action_at_point(&layout, &model, point, false),
        Some(UiAction::ToggleBrowserDuplicateCleanupMode)
    );
}
