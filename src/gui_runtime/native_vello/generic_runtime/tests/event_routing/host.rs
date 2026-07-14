use super::super::*;
use crate::gui::{
    focus::FocusSurface,
    input::{KeyCode, KeyPress},
    shortcuts::ShortcutResolution,
};
use crate::runtime::{RuntimeHostCapabilities, RuntimeInputHost};
use crate::widgets::{TextEditCommand, WidgetKey};

#[test]
fn generic_core_routes_pointer_and_key_input_to_host_messages() {
    let bridge = demo_bridge();
    let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
    let button_point = core
        .runtime
        .layout()
        .rects
        .get(&11)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("button should be laid out");

    assert!(
        core.route_pointer_press(button_point, PointerButton::Primary)
            .routed
    );
    assert!(
        core.route_pointer_release(button_point, PointerButton::Primary)
            .routed
    );
    assert_eq!(core.runtime.bridge().state.count, 1);

    let input_point = core
        .runtime
        .layout()
        .rects
        .get(&12)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("text input should be laid out");
    assert!(
        core.route_pointer_press(input_point, PointerButton::Primary)
            .routed
    );
    assert!(core.route_character('R').routed);
    assert!(core.route_character(' ').routed);
    assert!(core.route_widget_key(WidgetKey::Enter).routed);
    assert_eq!(core.runtime.bridge().state.name, "R ");
}

#[test]
fn nested_button_activation_survives_surface_refresh_between_press_and_release() {
    let bridge = demo_bridge();
    let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
    let button_point = core
        .runtime
        .layout()
        .rects
        .get(&11)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("button should be laid out");

    assert!(
        core.route_pointer_press(button_point, PointerButton::Primary)
            .routed
    );
    core.runtime.refresh();
    assert!(
        core.route_pointer_release(button_point, PointerButton::Primary)
            .routed
    );

    assert_eq!(core.runtime.bridge().state.count, 1);
}

#[test]
fn generic_core_routes_text_edit_commands_only_to_text_inputs() {
    let bridge = demo_bridge();
    let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
    let button_point = core
        .runtime
        .layout()
        .rects
        .get(&11)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("button should be laid out");

    assert!(
        core.route_pointer_press(button_point, PointerButton::Primary)
            .routed
    );
    assert!(!core.route_text_edit(TextEditCommand::SelectAll).routed);

    let input_point = core
        .runtime
        .layout()
        .rects
        .get(&12)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("text input should be laid out");
    assert!(
        core.route_pointer_press(input_point, PointerButton::Primary)
            .routed
    );
    assert!(core.route_text_edit(TextEditCommand::SelectAll).routed);
}

#[test]
fn focused_text_input_typing_preempts_host_shortcuts() {
    let bridge = ShortcutDemoBridge::default();
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        bridge,
        Vector2::new(320.0, 40.0),
    );
    focus_demo_text_input(&mut runner.core);

    let mut outcome = GenericRouteOutcome::default();
    assert!(runner.route_focused_text_input_before_shortcuts(KeyCode::E, Some("e"), &mut outcome,));

    assert_eq!(runner.core.runtime.bridge().state.name, "e");
    assert_eq!(runner.core.runtime.bridge().state.count, 0);
}

#[test]
fn handled_shortcut_text_does_not_enter_newly_focused_text_input() {
    let bridge = FocusTextInputShortcutBridge::default();
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        bridge,
        Vector2::new(320.0, 40.0),
    );

    let mut outcome = runner.core.route_key_press(
        KeyPress::new(KeyCode::Backquote),
        WidgetKey::from_key_code(KeyCode::Backquote),
    );
    assert!(outcome.routed);
    assert!(runner.core.has_focused_text_input());
    assert!(!runner.route_text_input_after_unhandled_keypress("§", &mut outcome));

    assert_eq!(runner.core.runtime.bridge().state.name, "");
}

#[test]
fn focused_text_input_routes_all_scalars_from_one_text_event() {
    let bridge = ShortcutDemoBridge::default();
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        bridge,
        Vector2::new(320.0, 40.0),
    );
    focus_demo_text_input(&mut runner.core);

    let mut outcome = GenericRouteOutcome::default();
    assert!(
        runner.route_focused_text_input_before_shortcuts(KeyCode::E, Some("éx"), &mut outcome,)
    );

    assert_eq!(runner.core.runtime.bridge().state.name, "éx");
    assert_eq!(runner.core.runtime.bridge().state.count, 0);
}

#[test]
fn focused_text_input_backspace_preempts_host_shortcuts() {
    let bridge = ShortcutDemoBridge::default();
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        bridge,
        Vector2::new(320.0, 40.0),
    );
    focus_demo_text_input(&mut runner.core);

    let mut type_outcome = GenericRouteOutcome::default();
    assert!(runner.route_focused_text_input_before_shortcuts(
        KeyCode::E,
        Some("e"),
        &mut type_outcome,
    ));
    assert_eq!(runner.core.runtime.bridge().state.name, "e");

    let mut backspace_outcome = GenericRouteOutcome::default();
    assert!(runner.route_focused_text_input_before_shortcuts(
        KeyCode::Backspace,
        None,
        &mut backspace_outcome,
    ));

    assert_eq!(runner.core.runtime.bridge().state.name, "");
    assert_eq!(runner.core.runtime.bridge().state.count, 0);
}

#[test]
fn focused_text_input_tab_routes_completion_before_host_shortcuts() {
    let bridge = ShortcutDemoBridge::default();
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        bridge,
        Vector2::new(320.0, 40.0),
    );
    focus_demo_text_input(&mut runner.core);

    let mut type_outcome = GenericRouteOutcome::default();
    assert!(runner.route_focused_text_input_before_shortcuts(
        KeyCode::E,
        Some("e"),
        &mut type_outcome,
    ));
    let mut tab_outcome = GenericRouteOutcome::default();
    assert!(
        runner.route_focused_text_input_before_shortcuts(KeyCode::Tab, None, &mut tab_outcome,)
    );

    assert_eq!(runner.core.runtime.bridge().state.name, "e");
    assert_eq!(runner.core.runtime.bridge().state.count, 0);
}

#[test]
fn generic_core_routes_second_nearby_press_as_double_click() {
    let bridge = CanvasBridge::default();
    let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
    let canvas_point = core
        .runtime
        .layout()
        .rects
        .get(&21)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("canvas should be laid out");

    assert!(
        core.route_pointer_press(canvas_point, PointerButton::Primary)
            .routed
    );
    assert!(
        core.route_pointer_release(canvas_point, PointerButton::Primary)
            .routed
    );
    assert!(
        core.route_pointer_press(canvas_point, PointerButton::Primary)
            .routed
    );

    assert_eq!(core.runtime.bridge().text, "double");
}

#[test]
fn refresh_restores_focused_canvas_without_emitting_host_focus_message() {
    let bridge = FocusRefreshCanvasBridge::default();
    let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
    let canvas_point = core
        .runtime
        .layout()
        .rects
        .get(&22)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("canvas should be laid out");

    assert!(
        core.route_pointer_press(canvas_point, PointerButton::Primary)
            .routed
    );
    assert_eq!(core.runtime.bridge().text, "focus;");
    core.runtime.bridge_mut().text.clear();

    core.runtime.refresh();

    assert_eq!(core.runtime.bridge().text, "");
}

#[derive(Default)]
struct ShortcutDemoBridge {
    state: DemoState,
}

#[derive(Default)]
struct FocusTextInputShortcutBridge {
    state: DemoState,
}

#[derive(Default)]
struct FocusRefreshCanvasBridge {
    text: String,
}

impl RuntimeBridge<String> for FocusRefreshCanvasBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<String>> {
        Arc::new(UiSurface::new(SurfaceNode::canvas_mapped(
            22,
            WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
            |message| match message {
                CanvasMessage::Input {
                    input: WidgetInput::FocusChanged(true),
                } => String::from("focus;"),
                _ => String::new(),
            },
        )))
    }

    fn reduce_message(&mut self, message: String) {
        self.text.push_str(&message);
    }
}

impl RuntimeBridge<DemoMessage> for ShortcutDemoBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&self.state)
    }

    fn reduce_message(&mut self, message: DemoMessage) {
        match message {
            DemoMessage::Increment => self.state.count += 1,
            DemoMessage::Rename(name) => self.state.name = name,
        }
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, DemoMessage> {
        RuntimeHostCapabilities::new().with_input()
    }
}

impl RuntimeInputHost<DemoMessage> for ShortcutDemoBridge {
    fn resolve_key_press(
        &mut self,
        _pending_chord: Option<KeyPress>,
        press: KeyPress,
        _focus: FocusSurface,
    ) -> ShortcutResolution<DemoMessage> {
        match press.key {
            KeyCode::Backspace | KeyCode::E => ShortcutResolution::action(DemoMessage::Increment),
            _ => ShortcutResolution::unhandled(),
        }
    }
}

impl RuntimeBridge<DemoMessage> for FocusTextInputShortcutBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&self.state)
    }

    fn update(&mut self, message: DemoMessage) -> Command<DemoMessage> {
        match message {
            DemoMessage::Increment => Command::focus(12),
            DemoMessage::Rename(name) => {
                self.state.name = name;
                Command::none()
            }
        }
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, DemoMessage> {
        RuntimeHostCapabilities::new().with_input()
    }
}

impl RuntimeInputHost<DemoMessage> for FocusTextInputShortcutBridge {
    fn resolve_key_press(
        &mut self,
        _pending_chord: Option<KeyPress>,
        press: KeyPress,
        _focus: FocusSurface,
    ) -> ShortcutResolution<DemoMessage> {
        match press.key {
            KeyCode::Backquote => ShortcutResolution::action(DemoMessage::Increment),
            _ => ShortcutResolution::unhandled(),
        }
    }
}

fn focus_demo_text_input<Bridge>(core: &mut GenericNativeRuntimeCore<Bridge, DemoMessage>)
where
    Bridge: RuntimeBridge<DemoMessage>,
{
    let input_point = core
        .runtime
        .layout()
        .rects
        .get(&12)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("text input should be laid out");
    assert!(
        core.route_pointer_press(input_point, PointerButton::Primary)
            .routed
    );
}

#[test]
fn second_press_falls_back_to_normal_press_when_widget_ignores_double_click() {
    let bridge = demo_bridge();
    let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
    let button_point = core
        .runtime
        .layout()
        .rects
        .get(&11)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("button should be laid out");

    assert!(
        core.route_pointer_press(button_point, PointerButton::Primary)
            .routed
    );
    assert!(
        core.route_pointer_release(button_point, PointerButton::Primary)
            .routed
    );
    assert!(
        core.route_pointer_press(button_point, PointerButton::Primary)
            .routed
    );
    assert!(
        core.route_pointer_release(button_point, PointerButton::Primary)
            .routed
    );

    assert_eq!(core.runtime.bridge().state.count, 2);
}
