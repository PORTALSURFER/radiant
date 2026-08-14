use super::*;

#[test]
fn timeline_runtime_cursor_motion_uses_paint_only_redraw_after_hover_enter() {
    let bridge = radiant::app(TimelineEditorState::default())
        .view(project_surface)
        .update(update)
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(860.0, 460.0));
    let geometry = TimelineGeometry::new(Rect::from_min_size(
        Point::new(16.0, 58.0),
        Vector2::new(828.0, 252.0),
    ));

    let enter = runtime.dispatch_pointer_move_with_outcome(Point::new(
        geometry.x_for_beat(20),
        geometry.lane_rect(1).center().y,
    ));
    assert!(enter.routed());
    assert!(enter.needs_scene_rebuild());

    let moved = runtime.dispatch_pointer_move_with_outcome(Point::new(
        geometry.x_for_beat(24) + 2.5,
        geometry.lane_rect(1).center().y,
    ));
    assert!(moved.routed());
    assert!(moved.paint_only_requested);
    assert!(!moved.needs_scene_rebuild());
}

#[test]
fn timeline_editor_routes_surface_messages_through_runtime() {
    let bridge = radiant::app(TimelineEditorState::default())
        .view(project_surface)
        .update(update)
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(860.0, 460.0));

    assert!(runtime.surface().find_widget(TIMELINE_WIDGET_ID).is_some());
    assert!(runtime.surface().find_widget(18).is_some());
    assert!(
        runtime
            .surface()
            .keyboard_focus_order()
            .contains(&TIMELINE_WIDGET_ID)
    );

    let geometry = TimelineGeometry::new(Rect::from_min_size(
        Point::new(16.0, 58.0),
        Vector2::new(828.0, 252.0),
    ));
    let target = Point::new(geometry.x_for_beat(48), geometry.lane_rect(0).center().y);
    assert!(runtime.dispatch_input(
        TIMELINE_WIDGET_ID,
        WidgetInput::PointerPress {
            position: target,
            button: PointerButton::Primary,
            modifiers: Default::default(),
            timestamp: None,
        },
    ));
    assert!(runtime.dispatch_input(
        TIMELINE_WIDGET_ID,
        WidgetInput::pointer_move(Point::new(geometry.x_for_beat(56), target.y)),
    ));
    assert!(runtime.dispatch_input(
        TIMELINE_WIDGET_ID,
        WidgetInput::PointerRelease {
            position: Point::new(geometry.x_for_beat(56), target.y),
            button: PointerButton::Primary,
            modifiers: Default::default(),
            timestamp: None,
        },
    ));

    let status = status_text(&runtime);
    assert!(status.contains("created clip"));
}

#[test]
fn timeline_editor_drag_preview_does_not_refresh_until_release() {
    let bridge = radiant::runtime::declarative_runtime_bridge(
        TimelineEditorState::default(),
        |state: &mut TimelineEditorState| {
            std::sync::Arc::new(project_surface(state).into_surface())
        },
        |state: &mut TimelineEditorState, message| update(state, message),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(860.0, 460.0));
    let geometry = TimelineGeometry::new(Rect::from_min_size(
        Point::new(16.0, 58.0),
        Vector2::new(828.0, 252.0),
    ));
    let press = Point::new(geometry.x_for_beat(4), geometry.lane_rect(0).center().y);
    let _ = runtime.dispatch_pointer_move_with_outcome(press);

    assert!(runtime.dispatch_input(
        TIMELINE_WIDGET_ID,
        WidgetInput::PointerPress {
            position: press,
            button: PointerButton::Primary,
            modifiers: Default::default(),
            timestamp: None,
        },
    ));
    let counters_before = runtime.refresh_counters();
    let state_before = runtime.bridge().state().clone();
    let surface_before = timeline_surface(&state_before);

    for position in [
        Point::new(geometry.x_for_beat(20), geometry.lane_rect(2).center().y),
        Point::new(geometry.x_for_beat(24), geometry.lane_rect(3).center().y),
        Point::new(geometry.x_for_beat(28), geometry.lane_rect(2).center().y),
    ] {
        let outcome = runtime.dispatch_pointer_move_with_outcome(position);
        assert!(outcome.routed());
        assert!(outcome.paint_only_requested);
        assert!(!outcome.needs_scene_rebuild());
        assert_eq!(runtime.bridge().state(), &state_before);
        assert_eq!(timeline_surface(runtime.bridge().state()), surface_before);
        assert_eq!(runtime.refresh_counters(), counters_before);
    }

    assert!(runtime.dispatch_input(
        TIMELINE_WIDGET_ID,
        WidgetInput::PointerRelease {
            position: Point::new(geometry.x_for_beat(28), geometry.lane_rect(2).center().y),
            button: PointerButton::Primary,
            modifiers: Default::default(),
            timestamp: None,
        },
    ));

    let state_after = runtime.bridge().state();
    assert_clip(state_after, 1, 2, BeatRange { start: 24, end: 40 });
    assert_eq!(
        state_after.feedback.revision,
        state_before.feedback.revision + 1
    );
    assert_eq!(
        runtime.refresh_counters().application_projection,
        counters_before.application_projection + 1
    );
}

#[test]
fn timeline_editor_deletes_selected_clip_from_toolbar() {
    let bridge = radiant::app(TimelineEditorState::default())
        .view(project_surface)
        .update(update)
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(860.0, 460.0));

    assert!(runtime.focus_widget(32));
    assert!(runtime.dispatch_input(32, WidgetInput::key_press(WidgetKey::Enter)));

    let status = status_text(&runtime);
    assert!(status.contains("clips 3"));
    assert!(status.contains("deleted clip 2"));
}
