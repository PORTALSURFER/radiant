use super::*;

fn button_label<Message>(surface: &UiSurface<Message>, widget_id: u64) -> String {
    surface
        .find_widget(widget_id)
        .expect("widget exists")
        .widget_object()
        .as_any()
        .downcast_ref::<ButtonWidget>()
        .expect("widget is button")
        .props
        .label
        .to_string()
}

#[test]
fn app_shortcuts_dispatch_messages_before_focused_widget_key_routing() {
    let bridge = app(DemoState::default())
        .view(|state: &mut DemoState| {
            radiant::prelude::button(format!("Count {}", state.count))
                .message(DemoMessage::Increment)
                .id(10)
        })
        .shortcuts(|_, _, press, _| {
            if press == KeyPress::with_command(KeyCode::I) {
                ShortcutResolution::action(DemoMessage::Increment)
            } else if press == KeyPress::new(KeyCode::Enter) {
                ShortcutResolution::handled()
            } else {
                ShortcutResolution::unhandled()
            }
        })
        .update_with(|state, message, _context| {
            if matches!(message, DemoMessage::Increment) {
                state.count += 1;
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));

    assert!(runtime.dispatch_key_press(
        KeyPress::with_command(KeyCode::I),
        None,
        FocusSurface::None
    ));
    assert_eq!(button_label(runtime.surface(), 10), "Count 1");

    assert!(runtime.focus_widget(10));
    assert!(runtime.dispatch_key_press(
        KeyPress::new(KeyCode::Enter),
        Some(WidgetKey::Enter),
        FocusSurface::None
    ));
    assert_eq!(button_label(runtime.surface(), 10), "Count 1");

    assert!(runtime.dispatch_key_press(
        KeyPress::new(KeyCode::Space),
        Some(WidgetKey::Space),
        FocusSurface::None
    ));
    assert_eq!(button_label(runtime.surface(), 10), "Count 2");
}

#[test]
fn shortcut_layer_public_api_handles_modal_layers_and_dynamic_fallbacks() {
    let modal = ShortcutLayer::modal().bind(KeyPress::new(KeyCode::Escape), DemoMessage::Increment);
    assert_eq!(
        modal.resolve(KeyPress::new(KeyCode::Escape)),
        ShortcutResolution::action(DemoMessage::Increment)
    );
    assert_eq!(
        modal.resolve(KeyPress::new(KeyCode::N)),
        ShortcutResolution::handled()
    );

    let global = ShortcutLayer::new().bind(
        ShortcutGesture::with_command(KeyCode::A),
        DemoMessage::Increment,
    );
    assert_eq!(
        global.resolve_or_else(KeyPress::new(KeyCode::ArrowDown), || {
            ShortcutResolution::action(DemoMessage::Increment)
        }),
        ShortcutResolution::action(DemoMessage::Increment)
    );
}
