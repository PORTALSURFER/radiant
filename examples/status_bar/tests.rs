use super::{model::StatusBarState, paint::progress_frame, view::project_surface};
use radiant::{
    layout::{Point, Rect, Vector2},
    prelude::{IntoView, StatusLineLog},
    theme::ThemeTokens,
};

#[test]
fn status_line_log_keeps_latest_bounded_message() {
    let mut log = StatusLineLog::new(3);

    log.publish("button", "pressed");
    log.publish("worker", "started");
    log.publish("animation", "stopped");

    assert_eq!(log.len(), 3);
    assert_eq!(log.latest(), "animation: stopped");
    assert_eq!(
        log.recent_lines(),
        vec![
            "animation: stopped".to_string(),
            "worker: started".to_string(),
            "button: pressed".to_string()
        ]
    );
}

#[test]
fn status_bar_state_tracks_actions_toggles_workers_and_progress() {
    let mut state = StatusBarState::default();

    state.record_action();
    state.set_autosave(false);
    let worker = state.start_worker("preview");
    state.tick();
    assert!(state.aggregate_progress() > 0.0);
    state.finish_worker(worker);
    state.toggle_animation();

    assert_eq!(state.action_count, 1);
    assert!(!state.autosave);
    assert_eq!(state.active_count(), 0);
    assert_eq!(state.completed_workers, 1);
    assert!(state.animation_running);
    assert_eq!(state.log.latest_line(), "animation: started");
    assert_eq!(state.log.latest(), "animation: started");
    assert_eq!(
        state.status_segments().center,
        "Autosave off | actions 1 | workers 1"
    );
}

#[test]
fn status_bar_allows_concurrent_worker_progress() {
    let mut state = StatusBarState::default();
    let first = state.start_worker("first");
    let second = state.start_worker("second");

    state.tick();
    state.finish_worker(first);

    assert_eq!(state.active_count(), 1);
    assert_eq!(state.completed_workers, 1);
    assert!(state.aggregate_progress() < 1.0);

    state.finish_worker(second);
    assert_eq!(state.active_count(), 0);
    assert_eq!(state.aggregate_progress(), 1.0);
}

#[test]
fn status_bar_projects_controls_and_status_strip() {
    let mut state = StatusBarState::default();
    let surface = project_surface(&mut state).into_surface();

    assert!(surface.find_widget(10).is_some());
    assert!(surface.find_widget(11).is_some());
    assert!(surface.find_widget(12).is_some());
    assert!(surface.find_widget(13).is_some());
    assert!(surface.find_widget(14).is_some());
    assert!(surface.find_widget(15).is_some());
    assert!(surface.keyboard_focus_order().len() >= 6);
}

#[test]
fn status_bar_paints_progress_inside_status_strip() {
    let mut state = StatusBarState::default();
    state.start_worker("preview");
    state.tick();

    let frame = progress_frame(
        &state,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(160.0, 18.0)),
        &ThemeTokens::default(),
    );

    assert!(frame.primitives.len() >= 4);
}
