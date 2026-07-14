use super::*;
use radiant::prelude::IntoView as _;

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

struct WrappedScene(radiant::prelude::ViewNode<DemoMessage>);

impl radiant::prelude::IntoView<DemoMessage> for WrappedScene {
    fn into_projection(self) -> radiant::prelude::ViewProjection<DemoMessage> {
        radiant::prelude::IntoView::into_projection(self.0)
    }
}

#[test]
fn app_shortcuts_dispatch_messages_before_focused_widget_key_routing() {
    let bridge = app(DemoState::default())
        .view(|state: &DemoState| {
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
        .handle_message(|state, message, _context| {
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
fn scene_shortcuts_dispatch_messages_before_focused_widget_key_routing() {
    let bridge = app(DemoState::default())
        .view(|state: &DemoState| {
            radiant::prelude::scene(
                radiant::prelude::button(format!("Count {}", state.count))
                    .message(DemoMessage::Increment)
                    .id(10),
            )
            .shortcuts(
                ShortcutCatalog::new().layer(
                    ShortcutLayer::new()
                        .bind(KeyPress::with_command(KeyCode::I), DemoMessage::Increment)
                        .bind(KeyPress::new(KeyCode::Enter), DemoMessage::Noop),
                ),
            )
            .into_view()
        })
        .handle_message(|state, message, _context| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Noop => {}
            _ => {}
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
}

#[test]
fn wrapped_scene_projection_preserves_shortcuts() {
    let bridge = app(DemoState::default())
        .view(|state: &DemoState| {
            WrappedScene(
                radiant::prelude::scene(
                    radiant::prelude::button(format!("Count {}", state.count))
                        .message(DemoMessage::Increment)
                        .id(10),
                )
                .shortcuts(
                    ShortcutCatalog::new().layer(
                        ShortcutLayer::new()
                            .bind(KeyPress::with_command(KeyCode::I), DemoMessage::Increment),
                    ),
                )
                .into_view(),
            )
        })
        .handle_message(|state, message, _context| match message {
            DemoMessage::Increment => state.count += 1,
            _ => {}
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));

    assert!(runtime.dispatch_key_press(
        KeyPress::with_command(KeyCode::I),
        None,
        FocusSurface::None
    ));
    assert_eq!(button_label(runtime.surface(), 10), "Count 1");
}

#[test]
fn nested_scene_projection_preserves_presentation_and_shortcut_precedence() {
    let bridge = app(DemoState::default())
        .view(|state: &DemoState| {
            radiant::prelude::scene(
                radiant::prelude::scene(
                    radiant::prelude::button(format!("Count {}", state.count))
                        .message(DemoMessage::Increment)
                        .id(10),
                )
                .frame_clock(
                    radiant::prelude::FrameClock::message(DemoMessage::Increment)
                        .when(|_state: &mut DemoState| true),
                )
                .shortcuts(
                    ShortcutCatalog::new().layer(
                        ShortcutLayer::new()
                            .bind(KeyPress::with_command(KeyCode::I), DemoMessage::Increment),
                    ),
                )
                .into_view(),
            )
            .frame_clock(
                radiant::prelude::FrameClock::message(DemoMessage::Noop)
                    .when(|_state: &mut DemoState| true),
            )
            .shortcuts(ShortcutCatalog::new().layer(
                ShortcutLayer::new().bind(KeyPress::with_command(KeyCode::I), DemoMessage::Noop),
            ))
            .into_view()
            .into_projection()
        })
        .handle_message(|state, message, _context| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Noop => {}
            _ => {}
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));

    assert!(runtime.dispatch_key_press(
        KeyPress::with_command(KeyCode::I),
        None,
        FocusSurface::None
    ));
    assert_eq!(button_label(runtime.surface(), 10), "Count 0");

    assert!(runtime.host_queue_animation_frame());
    assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 1);
    assert_eq!(button_label(runtime.surface(), 10), "Count 1");
}

#[test]
fn scene_modal_shortcut_layer_consumes_unmatched_keys() {
    let bridge = app(DemoState::default())
        .view(|state: &DemoState| {
            radiant::prelude::scene(
                radiant::prelude::button(format!("Count {}", state.count))
                    .message(DemoMessage::Increment)
                    .id(10),
            )
            .shortcuts(
                ShortcutCatalog::new().layer(ShortcutLayer::modal_escape(DemoMessage::Increment)),
            )
            .into_view()
        })
        .handle_message(|state, message, _context| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Noop => {}
            _ => {}
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));

    assert!(runtime.dispatch_key_press(KeyPress::new(KeyCode::Escape), None, FocusSurface::None));
    assert_eq!(button_label(runtime.surface(), 10), "Count 1");

    assert!(runtime.focus_widget(10));
    assert!(runtime.dispatch_key_press(
        KeyPress::new(KeyCode::Space),
        Some(WidgetKey::Space),
        FocusSurface::None
    ));
    assert_eq!(button_label(runtime.surface(), 10), "Count 1");
}

#[test]
fn scene_shortcut_fallback_handles_dynamic_keys() {
    let bridge = app(DemoState::default())
        .view(|state: &DemoState| {
            radiant::prelude::scene(
                radiant::prelude::button(format!("Count {}", state.count))
                    .message(DemoMessage::Increment)
                    .id(10),
            )
            .shortcuts(ShortcutCatalog::new().fallback(|press| {
                if press == KeyPress::new(KeyCode::ArrowDown) {
                    ShortcutResolution::action(DemoMessage::Increment)
                } else {
                    ShortcutResolution::unhandled()
                }
            }))
            .into_view()
        })
        .handle_message(|state, message, _context| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Noop => {}
            _ => {}
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));

    assert!(runtime.dispatch_key_press(
        KeyPress::new(KeyCode::ArrowDown),
        None,
        FocusSurface::None
    ));
    assert_eq!(button_label(runtime.surface(), 10), "Count 1");
}

#[test]
fn scene_shortcuts_fall_back_to_app_builder_shortcuts_when_unhandled() {
    let bridge = app(DemoState::default())
        .view(|state: &DemoState| {
            radiant::prelude::scene(
                radiant::prelude::button(format!("Count {}", state.count))
                    .message(DemoMessage::Increment)
                    .id(10),
            )
            .shortcuts(ShortcutCatalog::new().layer(
                ShortcutLayer::new().bind(KeyPress::with_command(KeyCode::I), DemoMessage::Noop),
            ))
            .into_view()
        })
        .shortcuts(|_, _, press, _| {
            if press == KeyPress::new(KeyCode::ArrowDown) {
                ShortcutResolution::action(DemoMessage::Increment)
            } else {
                ShortcutResolution::unhandled()
            }
        })
        .handle_message(|state, message, _context| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Noop => {}
            _ => {}
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));

    assert!(runtime.dispatch_key_press(
        KeyPress::new(KeyCode::ArrowDown),
        None,
        FocusSurface::None
    ));
    assert_eq!(button_label(runtime.surface(), 10), "Count 1");
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

    let stack = ShortcutStack::new().push(modal).push(global).push_when(
        false,
        ShortcutLayer::new().bind(KeyPress::new(KeyCode::N), DemoMessage::Increment),
    );
    assert_eq!(stack.layers().len(), 2);
    assert_eq!(
        stack.resolve(KeyPress::new(KeyCode::N)),
        ShortcutResolution::handled()
    );
}

#[test]
fn undo_history_public_api_wraps_state_mutations_and_shortcuts() {
    let mut history = radiant::gui::undo::UndoHistory::new();
    let mut value = String::from("one");

    assert!(history.apply("rename", &mut value, |value| {
        *value = String::from("two");
    }));
    assert_eq!(value, "two");

    let undo = history.undo(&value).expect("undo checkpoint");
    value = undo.state;
    assert_eq!(value, "one");

    let redo = history.redo(&value).expect("redo checkpoint");
    value = redo.state;
    assert_eq!(value, "two");
    assert_eq!(
        radiant::gui::undo::UndoRedoIntent::from_key_press(KeyPress::with_command(KeyCode::Z)),
        Some(radiant::gui::undo::UndoRedoIntent::Undo)
    );
}
