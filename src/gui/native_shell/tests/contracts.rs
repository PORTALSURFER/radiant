use super::*;

#[test]
fn focused_rows_do_not_enable_idle_animation_when_transport_is_stopped() {
    let mut state = NativeShellState::new();
    let mut model = crate::app::AppModel::default();
    model.transport_running = false;
    model
        .browser
        .rows
        .push(crate::app::BrowserRowModel::new(0, "kick", 1, false, true));
    state.sync_from_model(&model);
    state.sync_from_model(&model);
    assert!(!state.needs_animation());

    let mut idle_model = crate::app::AppModel::default();
    idle_model.transport_running = false;
    let mut playing_model = crate::app::AppModel::default();
    playing_model.transport_running = true;
    state.sync_from_model(&playing_model);
    assert!(state.needs_animation());
    state.sync_from_model(&idle_model);
    assert!(!state.needs_animation());
}

#[test]
fn long_browser_labels_are_truncated_with_ellipsis() {
    let layout = ShellLayout::build(Vector2::new(620.0, 420.0));
    let mut state = NativeShellState::new();
    let mut model = crate::app::AppModel::default();
    model.browser.rows.push(crate::app::BrowserRowModel::new(
        0,
        "this_is_a_very_long_browser_row_label_that_should_truncate_in_native_shell_rendering_and_is_intentionally_longer_than_any_practical_row_width_even_on_narrow_compact_views.wav",
        1,
        false,
        false,
    ));
    state.sync_from_model(&model);
    let frame = state.build_frame(&layout, &model);
    let truncated = frame
        .text_runs
        .iter()
        .find(|run| run.text.starts_with("this_is_a"))
        .map(|run| run.text.as_str())
        .unwrap_or_default();
    assert!(truncated.ends_with("..."));
}

#[test]
fn canonical_frame_rebuild_is_deterministic_across_tiers() {
    let mut state = NativeShellState::new();
    let model = canonical_shell_model();
    state.sync_from_model(&model);
    for viewport in [
        Vector2::new(820.0, 520.0),
        Vector2::new(1280.0, 720.0),
        Vector2::new(2300.0, 1080.0),
    ] {
        let layout = ShellLayout::build(viewport);
        let frame_a = state.build_frame(&layout, &model);
        let frame_b = state.build_frame(&layout, &model);
        assert_eq!(frame_a, frame_b);
        assert!(!frame_a.primitives.is_empty());
        assert!(!frame_a.text_runs.is_empty());
    }
}

#[test]
fn canonical_frame_contains_expected_sidebar_and_status_contract_text() {
    let mut state = NativeShellState::new();
    let model = canonical_shell_model();
    state.sync_from_model(&model);
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let frame = state.build_frame(&layout, &model);
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("Folders ("))
    );
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("entries"))
    );
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("rows: 48"))
    );
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("col: 2/3"))
    );
    assert!(frame.text_runs.iter().any(|run| run.text == "kick"));
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("36 items"))
    );
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("Loop engaged"))
    );
}
