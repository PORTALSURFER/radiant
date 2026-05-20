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
        },
    ));
    assert!(runtime.dispatch_input(
        TIMELINE_WIDGET_ID,
        WidgetInput::PointerRelease {
            position: Point::new(geometry.x_for_beat(56), target.y),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    ));

    let status = status_text(&runtime);
    assert!(status.contains("created clip"));
}

#[test]
fn timeline_editor_deletes_selected_clip_from_toolbar() {
    let bridge = radiant::app(TimelineEditorState::default())
        .view(project_surface)
        .update(update)
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(860.0, 460.0));

    assert!(runtime.focus_widget(32));
    assert!(runtime.dispatch_input(32, WidgetInput::KeyPress(WidgetKey::Enter)));

    let status = status_text(&runtime);
    assert!(status.contains("clips 3"));
    assert!(status.contains("deleted clip 2"));
}
