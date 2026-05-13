use super::*;

#[test]
fn surface_runtime_manages_focus_and_routes_keyboard_to_focused_widget() {
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

    assert_eq!(runtime.focused_widget(), None);
    assert_eq!(runtime.traverse_focus(FocusTraversal::Forward), Some(11));
    assert_eq!(runtime.focused_widget(), Some(11));
    assert_eq!(runtime.traverse_focus(FocusTraversal::Forward), Some(12));
    assert_eq!(runtime.traverse_focus(FocusTraversal::Backward), Some(11));
    assert_eq!(
        runtime.dispatch_focused_input(WidgetInput::KeyPress(WidgetKey::Enter)),
        Some(11)
    );

    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "Untitled (1)"
    );

    assert!(runtime.focus_widget(12));
    assert_eq!(
        runtime.dispatch_focused_input(WidgetInput::Character('Q')),
        Some(12)
    );
    runtime.clear_focus();
    assert_eq!(runtime.focused_widget(), None);
    assert_eq!(
        runtime.dispatch_focused_input(WidgetInput::Character('X')),
        None
    );

    assert_eq!(
        widget_ref::<TextInputWidget, _>(runtime.surface(), 12, "text input")
            .state
            .value,
        "Q"
    );
}

#[test]
fn surface_runtime_preserves_text_input_caret_selection_across_value_refreshes() {
    let bridge = declarative_runtime_bridge(
        DemoState {
            name: String::from("abcd"),
            ..DemoState::default()
        },
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::Increment | DemoMessage::CanvasInput(_) => {}
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    assert!(runtime.focus_widget(12));
    assert_eq!(
        runtime.dispatch_focused_input(WidgetInput::TextEdit(TextEditCommand::MoveHome {
            extend_selection: false,
        })),
        Some(12)
    );
    assert_eq!(
        runtime.dispatch_focused_input(WidgetInput::TextEdit(TextEditCommand::MoveRight {
            extend_selection: true,
        })),
        Some(12)
    );
    assert_eq!(runtime.focused_text_selection().as_deref(), Some("ab"));
    assert_eq!(
        runtime.dispatch_focused_input(WidgetInput::TextEdit(TextEditCommand::InsertText(
            String::from("z")
        ))),
        Some(12)
    );

    let input = widget_ref::<TextInputWidget, _>(runtime.surface(), 12, "text input");
    assert_eq!(input.state.value, "zcd");
    assert_eq!(input.state.caret, 1);
    assert_eq!(input.state.selection_anchor, 1);
}

#[test]
fn surface_runtime_reports_focused_text_input_only_when_editable() {
    let mut runtime = text_input_runtime(false, false);
    assert!(runtime.focus_widget(12));
    assert_eq!(runtime.focused_text_input_id(), Some(12));

    let mut read_only_runtime = text_input_runtime(false, true);
    assert!(read_only_runtime.focus_widget(12));
    assert_eq!(read_only_runtime.focused_text_input_id(), None);

    let mut disabled_runtime = text_input_runtime(true, false);
    assert!(!disabled_runtime.focus_widget(12));
    assert_eq!(disabled_runtime.focused_text_input_id(), None);
}

#[test]
fn surface_runtime_keeps_disabled_widgets_out_of_focus_order() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        |_state: &mut DemoState| {
            let mut disabled = ButtonWidget::new(
                11,
                "Disabled",
                WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
            );
            disabled.common.state.disabled = true;
            let enabled =
                ButtonWidget::new(12, "Enabled", WidgetSizing::fixed(Vector2::new(96.0, 28.0)));
            Arc::new(UiSurface::new(SurfaceNode::row(
                1,
                8.0,
                vec![
                    SurfaceChild::fill(SurfaceNode::static_widget(disabled)),
                    SurfaceChild::fill(SurfaceNode::static_widget(enabled)),
                ],
            )))
        },
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 40.0));

    assert_eq!(runtime.surface().keyboard_focus_order(), vec![12]);
    assert!(!runtime.focus_widget(11));
    assert_eq!(runtime.traverse_focus(FocusTraversal::Forward), Some(12));
}

#[test]
fn surface_runtime_clears_focus_when_refresh_removes_widget() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        |state: &mut DemoState| {
            let child = if state.count == 0 {
                SurfaceNode::text_input(
                    12,
                    state.name.clone(),
                    WidgetSizing::new(Vector2::new(120.0, 28.0), Vector2::new(180.0, 28.0)),
                    DemoMessage::Rename,
                )
            } else {
                SurfaceNode::static_widget(TextWidget::new(
                    10,
                    "Removed",
                    WidgetSizing::fixed(Vector2::new(120.0, 20.0)).with_baseline(14.0),
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
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 40.0));

    assert!(runtime.focus_widget(12));
    assert_eq!(runtime.focused_widget(), Some(12));

    runtime.dispatch_message(DemoMessage::Increment);

    assert_eq!(runtime.focused_widget(), None);
    assert!(
        runtime.surface().find_widget(12).is_none(),
        "the refreshed surface should no longer contain the focused widget"
    );
}

fn text_input_runtime(
    disabled: bool,
    read_only: bool,
) -> SurfaceRuntime<impl RuntimeBridge<DemoMessage>, DemoMessage> {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        move |_state: &mut DemoState| {
            let mut input = TextInputWidget::new(
                12,
                "editable",
                WidgetSizing::new(Vector2::new(120.0, 28.0), Vector2::new(180.0, 28.0)),
            );
            input.common.state.disabled = disabled;
            input.common.state.read_only = read_only;
            Arc::new(UiSurface::new(SurfaceNode::static_widget(input)))
        },
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::CanvasInput(_) => {}
        },
    );
    SurfaceRuntime::new(bridge, Vector2::new(220.0, 40.0))
}
