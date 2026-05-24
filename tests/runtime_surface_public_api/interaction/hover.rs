use super::*;

#[test]
fn surface_runtime_clears_hover_when_pointer_leaves_widget() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    assert_eq!(
        runtime.dispatch_event(Event::PointerMove {
            position: Point::new(150.0, 10.0),
        }),
        Some(11)
    );
    assert_eq!(runtime.hovered_widget(), Some(11));
    assert!(button_hovered(runtime.surface(), 11));

    assert_eq!(
        runtime.dispatch_event(Event::PointerMove {
            position: Point::new(410.0, 80.0),
        }),
        None
    );
    assert_eq!(runtime.hovered_widget(), None);
    assert!(!button_hovered(runtime.surface(), 11));
}

#[test]
fn surface_runtime_clears_hover_when_refresh_removes_widget() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        |state: &mut DemoState| {
            let child = if state.count == 0 {
                SurfaceNode::button(
                    11,
                    "Temporary",
                    WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
                    DemoMessage::Increment,
                )
            } else {
                SurfaceNode::static_widget(TextWidget::new(
                    12,
                    "Removed",
                    WidgetSizing::fixed(Vector2::new(96.0, 28.0)).with_baseline(18.0),
                ))
            };
            Arc::new(UiSurface::new(child))
        },
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(140.0, 40.0));

    assert_eq!(
        runtime.dispatch_event(Event::PointerMove {
            position: Point::new(12.0, 12.0),
        }),
        Some(11)
    );
    assert_eq!(runtime.hovered_widget(), Some(11));

    runtime.dispatch_message(DemoMessage::Increment);

    assert_eq!(runtime.hovered_widget(), None);
    assert!(
        runtime.surface().find_widget(11).is_none(),
        "the refreshed surface should no longer contain the hovered widget"
    );
}
