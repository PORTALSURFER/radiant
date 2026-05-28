use super::*;

#[test]
fn surface_runtime_preserves_captured_drag_state_across_repaint_refreshes() {
    let bridge = declarative_command_runtime_bridge(
        Vec::<DragHandleMessage>::new(),
        |_| {
            Arc::new(UiSurface::new(SurfaceNode::widget(
                DragHandleWidget::new(10, WidgetSizing::fixed(Vector2::new(24.0, 24.0))),
                WidgetMessageMapper::drag_handle(|message| message),
            )))
        },
        |messages: &mut Vec<DragHandleMessage>, message| {
            messages.push(message);
            Command::request_repaint()
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 120.0));

    assert_eq!(
        runtime.dispatch_event(Event::PointerPress {
            position: Point::new(12.0, 12.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        }),
        Some(10)
    );
    assert_eq!(
        runtime.dispatch_event(Event::PointerMove {
            position: Point::new(12.0, 72.0),
        }),
        Some(10)
    );

    assert_eq!(
        runtime.bridge().state().as_slice(),
        &[
            DragHandleMessage::Started {
                position: Point::new(12.0, 12.0),
            },
            DragHandleMessage::Moved {
                position: Point::new(12.0, 72.0),
            },
        ]
    );
}

#[test]
fn surface_runtime_routes_widget_input_and_reprojects_surface() {
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
    let input_bounds = runtime
        .layout()
        .rects
        .get(&12)
        .copied()
        .expect("text input should have layout bounds");
    let input_point = Point::new(
        input_bounds.min.x + input_bounds.width() * 0.5,
        input_bounds.min.y + input_bounds.height() * 0.5,
    );

    assert_eq!(runtime.widget_at(Point::new(150.0, 10.0)), Some(11));
    assert!(runtime.dispatch_input(12, WidgetInput::FocusChanged(true)));
    assert!(runtime.dispatch_input(12, WidgetInput::Character('F')));
    assert!(runtime.dispatch_input(11, WidgetInput::FocusChanged(true)));
    assert_eq!(
        runtime.dispatch_input_at(input_point, WidgetInput::FocusChanged(true)),
        Some(12)
    );
    assert!(runtime.dispatch_input(11, WidgetInput::KeyPress(WidgetKey::Enter)));

    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "F (1)"
    );
    assert_eq!(
        widget_ref::<TextInputWidget, _>(runtime.surface(), 12, "text input")
            .state
            .value,
        "F"
    );
}

#[derive(Clone, Debug, PartialEq)]
enum ReleaseRefreshMessage {
    Activate,
    Toggle(bool),
    Refresh,
}

#[derive(Default)]
struct ReleaseRefreshState {
    activations: usize,
    checked: bool,
    refreshes: usize,
}

#[test]
fn surface_runtime_preserves_badge_release_activation_across_refresh() {
    let bridge = declarative_runtime_bridge(
        ReleaseRefreshState::default(),
        |state: &mut ReleaseRefreshState| {
            Arc::new(UiSurface::new(SurfaceNode::badge(
                20,
                format!("Tag {}", state.refreshes),
                WidgetSizing::fixed(Vector2::new(72.0, 24.0)),
                ReleaseRefreshMessage::Activate,
            )))
        },
        |state: &mut ReleaseRefreshState, message| match message {
            ReleaseRefreshMessage::Activate => state.activations += 1,
            ReleaseRefreshMessage::Toggle(checked) => state.checked = checked,
            ReleaseRefreshMessage::Refresh => state.refreshes += 1,
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(96.0, 40.0));
    let point = Point::new(24.0, 12.0);

    assert_eq!(
        runtime.dispatch_event(Event::PointerPress {
            position: point,
            button: PointerButton::Primary,
            modifiers: Default::default(),
        }),
        Some(20)
    );
    runtime.dispatch_message(ReleaseRefreshMessage::Refresh);
    assert_eq!(runtime.bridge().state().activations, 0);
    assert_eq!(runtime.bridge().state().refreshes, 1);

    assert_eq!(
        runtime.dispatch_event(Event::PointerRelease {
            position: point,
            button: PointerButton::Primary,
            modifiers: Default::default(),
        }),
        Some(20)
    );
    assert_eq!(runtime.bridge().state().activations, 1);
}

#[test]
fn surface_runtime_preserves_toggle_release_activation_across_refresh() {
    let bridge = declarative_runtime_bridge(
        ReleaseRefreshState::default(),
        |state: &mut ReleaseRefreshState| {
            Arc::new(UiSurface::new(SurfaceNode::toggle_with_checked(
                20,
                format!("Loop {}", state.refreshes),
                state.checked,
                WidgetSizing::fixed(Vector2::new(96.0, 24.0)),
                ReleaseRefreshMessage::Toggle,
            )))
        },
        |state: &mut ReleaseRefreshState, message| match message {
            ReleaseRefreshMessage::Activate => state.activations += 1,
            ReleaseRefreshMessage::Toggle(checked) => state.checked = checked,
            ReleaseRefreshMessage::Refresh => state.refreshes += 1,
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 40.0));
    let point = Point::new(24.0, 12.0);

    assert_eq!(
        runtime.dispatch_event(Event::PointerPress {
            position: point,
            button: PointerButton::Primary,
            modifiers: Default::default(),
        }),
        Some(20)
    );
    runtime.dispatch_message(ReleaseRefreshMessage::Refresh);
    assert!(!runtime.bridge().state().checked);

    assert_eq!(
        runtime.dispatch_event(Event::PointerRelease {
            position: point,
            button: PointerButton::Primary,
            modifiers: Default::default(),
        }),
        Some(20)
    );
    assert!(runtime.bridge().state().checked);
}
