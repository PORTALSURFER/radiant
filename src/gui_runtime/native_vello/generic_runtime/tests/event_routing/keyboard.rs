use super::super::*;
use crate::{
    gui::{
        focus::FocusSurface,
        input::{InputTimestamp, KeyCode, KeyPress},
        shortcuts::ShortcutResolution,
        types::Rect,
    },
    layout::LayoutOutput,
    runtime::{
        PaintPrimitive, RuntimeBridge, RuntimeHostCapabilities, RuntimeInputHost, SurfaceNode,
        UiSurface, WidgetMessageMapper,
    },
    theme::ThemeTokens,
    widgets::{
        CanvasMessage, CanvasWidget, KeyboardModifiers, TextEditCommand, Widget, WidgetCommon,
        WidgetId, WidgetInput, WidgetKey, WidgetOutput, WidgetSizing,
    },
};
use std::sync::Arc;
use winit::keyboard::{KeyCode as WinitKeyCode, ModifiersState, PhysicalKey};

#[derive(Clone)]
struct FocusedKeyboardMetadataWidget {
    inner: CanvasWidget,
}

impl FocusedKeyboardMetadataWidget {
    fn new(id: WidgetId) -> Self {
        Self {
            inner: CanvasWidget::new(id, WidgetSizing::fixed(Vector2::new(160.0, 28.0))),
        }
    }
}

impl Widget for FocusedKeyboardMetadataWidget {
    fn common(&self) -> &WidgetCommon {
        Widget::common(&self.inner)
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        Widget::common_mut(&mut self.inner)
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        CanvasWidget::handle_input(&mut self.inner, bounds, input).map(WidgetOutput::typed)
    }

    fn accepts_text_input(&self) -> bool {
        true
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        Widget::append_paint(&self.inner, primitives, bounds, layout, theme);
    }
}

#[derive(Clone, Debug, PartialEq)]
enum KeyboardTimestampMessage {
    KeyPress {
        modifiers: KeyboardModifiers,
        repeat: bool,
        timestamp: Option<InputTimestamp>,
    },
    KeyRelease {
        key: WidgetKey,
        modifiers: KeyboardModifiers,
        timestamp: Option<InputTimestamp>,
    },
    Character {
        character: char,
        timestamp: Option<InputTimestamp>,
    },
    TextEdit(Option<InputTimestamp>),
    Ignored,
}

#[derive(Default)]
struct KeyboardTimestampBridge {
    messages: Vec<KeyboardTimestampMessage>,
}

impl RuntimeBridge<KeyboardTimestampMessage> for KeyboardTimestampBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<KeyboardTimestampMessage>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
            FocusedKeyboardMetadataWidget::new(90),
            WidgetMessageMapper::canvas(|message| match message {
                CanvasMessage::Input {
                    input:
                        WidgetInput::KeyPress {
                            modifiers,
                            repeat,
                            timestamp,
                            ..
                        },
                } => KeyboardTimestampMessage::KeyPress {
                    modifiers,
                    repeat,
                    timestamp,
                },
                CanvasMessage::Input {
                    input:
                        WidgetInput::KeyRelease {
                            key,
                            modifiers,
                            timestamp,
                        },
                } => KeyboardTimestampMessage::KeyRelease {
                    key,
                    modifiers,
                    timestamp,
                },
                CanvasMessage::Input {
                    input:
                        WidgetInput::Character {
                            character,
                            timestamp,
                        },
                } => KeyboardTimestampMessage::Character {
                    character,
                    timestamp,
                },
                CanvasMessage::Input {
                    input: WidgetInput::TextEdit { timestamp, .. },
                } => KeyboardTimestampMessage::TextEdit(timestamp),
                CanvasMessage::Input { .. } => KeyboardTimestampMessage::Ignored,
            }),
        )))
    }

    fn reduce_message(&mut self, message: KeyboardTimestampMessage) {
        if !matches!(message, KeyboardTimestampMessage::Ignored) {
            self.messages.push(message);
        }
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, KeyboardTimestampMessage> {
        RuntimeHostCapabilities::new().with_input()
    }
}

impl RuntimeInputHost<KeyboardTimestampMessage> for KeyboardTimestampBridge {
    fn resolve_key_press(
        &mut self,
        _pending_chord: Option<KeyPress>,
        press: KeyPress,
        _focus: FocusSurface,
    ) -> ShortcutResolution<KeyboardTimestampMessage> {
        if press.key == KeyCode::ArrowUp {
            ShortcutResolution::handled()
        } else {
            ShortcutResolution::unhandled()
        }
    }
}

#[test]
fn direct_physical_key_route_preserves_one_timestamp() {
    let timestamp = Some(InputTimestamp::capture());
    let mut core = GenericNativeRuntimeCore::new(
        KeyboardTimestampBridge::default(),
        Vector2::new(160.0, 28.0),
    );

    assert!(core.runtime.focus_widget(90));
    assert!(
        core.route_key_press_with_timestamp(
            KeyPress::new(KeyCode::Enter),
            Some(WidgetKey::Enter),
            KeyboardModifiers::default(),
            timestamp,
            false,
        )
        .routed
    );
    assert_eq!(
        core.runtime.bridge().messages,
        vec![KeyboardTimestampMessage::KeyPress {
            modifiers: KeyboardModifiers::default(),
            repeat: false,
            timestamp,
        }]
    );
}

#[test]
fn direct_physical_key_route_preserves_modifier_and_repeat_metadata() {
    let timestamp = Some(InputTimestamp::capture());
    let mut core = GenericNativeRuntimeCore::new(
        KeyboardTimestampBridge::default(),
        Vector2::new(160.0, 28.0),
    );

    assert!(core.runtime.focus_widget(90));
    assert!(
        core.route_key_press_with_timestamp(
            KeyPress {
                key: KeyCode::ArrowRight,
                command: true,
                control: true,
                shift: true,
                alt: true,
            },
            Some(WidgetKey::ArrowRight),
            KeyboardModifiers {
                command: true,
                control: true,
                shift: true,
                alt: true,
            },
            timestamp,
            true,
        )
        .routed
    );
    assert_eq!(
        core.runtime.bridge().messages,
        vec![KeyboardTimestampMessage::KeyPress {
            modifiers: KeyboardModifiers {
                command: true,
                control: true,
                shift: true,
                alt: true,
            },
            repeat: true,
            timestamp,
        }]
    );
}

#[test]
fn logical_deletion_fallback_preserves_modifier_and_repeat_metadata() {
    let timestamp = Some(InputTimestamp::capture());
    let modifiers = KeyboardModifiers {
        command: true,
        control: true,
        shift: true,
        alt: true,
    };
    let mut core = GenericNativeRuntimeCore::new(
        KeyboardTimestampBridge::default(),
        Vector2::new(160.0, 28.0),
    );

    assert!(core.runtime.focus_widget(90));
    assert!(
        core.route_widget_key_with_metadata(WidgetKey::Backspace, modifiers, true, timestamp)
            .routed
    );
    assert_eq!(
        core.runtime.bridge().messages,
        vec![KeyboardTimestampMessage::KeyPress {
            modifiers,
            repeat: true,
            timestamp,
        }]
    );
}

#[cfg(not(target_os = "macos"))]
#[test]
fn unhandled_native_control_keeps_host_and_widget_modifier_views_distinct() {
    let native_modifiers = ModifiersState::CONTROL;
    let host_press = keypress_from_input(KeyCode::ArrowRight, native_modifiers);
    assert!(host_press.command);
    assert!(!host_press.control);
    let widget_modifiers = keyboard_modifiers_from_winit(native_modifiers);
    assert_eq!(
        widget_modifiers,
        KeyboardModifiers {
            command: false,
            control: true,
            shift: false,
            alt: false,
        }
    );

    let timestamp = Some(InputTimestamp::capture());
    let mut core = GenericNativeRuntimeCore::new(
        KeyboardTimestampBridge::default(),
        Vector2::new(160.0, 28.0),
    );
    assert!(core.runtime.focus_widget(90));
    assert!(
        core.route_key_press_with_timestamp(
            host_press,
            Some(WidgetKey::ArrowRight),
            widget_modifiers,
            timestamp,
            true,
        )
        .routed
    );
    assert_eq!(
        core.runtime.bridge().messages,
        vec![KeyboardTimestampMessage::KeyPress {
            modifiers: widget_modifiers,
            repeat: true,
            timestamp,
        }]
    );
}

#[test]
fn handled_native_host_shortcut_does_not_reach_focused_widget() {
    let native_modifiers = ModifiersState::CONTROL;
    let host_press = keypress_from_input(KeyCode::ArrowUp, native_modifiers);
    let widget_modifiers = keyboard_modifiers_from_winit(native_modifiers);
    let mut core = GenericNativeRuntimeCore::new(
        KeyboardTimestampBridge::default(),
        Vector2::new(160.0, 28.0),
    );
    assert!(core.runtime.focus_widget(90));

    assert!(
        core.route_key_press_with_timestamp(
            host_press,
            Some(WidgetKey::ArrowUp),
            widget_modifiers,
            Some(InputTimestamp::capture()),
            false,
        )
        .routed
    );
    assert!(core.runtime.bridge().messages.is_empty());
}

#[test]
fn native_physical_key_release_routes_once_with_metadata() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        KeyboardTimestampBridge::default(),
        Vector2::new(160.0, 28.0),
    );
    runner.input.modifiers = ModifiersState::CONTROL
        | ModifiersState::SUPER
        | ModifiersState::SHIFT
        | ModifiersState::ALT;
    let expected_modifiers = KeyboardModifiers {
        command: true,
        control: true,
        shift: true,
        alt: true,
    };

    assert!(runner.core.runtime.focus_widget(90));
    assert!(
        runner
            .route_native_key_release(PhysicalKey::Code(WinitKeyCode::ArrowDown))
            .expect("supported physical release should produce a route outcome")
            .routed
    );
    let messages = &runner.core.runtime.bridge().messages;
    let Some(KeyboardTimestampMessage::KeyRelease {
        key,
        modifiers,
        timestamp,
    }) = messages.first()
    else {
        panic!("native release should deliver one key-release message");
    };
    assert_eq!(*key, WidgetKey::ArrowDown);
    assert_eq!(*modifiers, expected_modifiers);
    assert!(timestamp.is_some());
    assert_eq!(messages.len(), 1);
}

#[test]
fn unsupported_or_unfocused_key_release_is_not_routed() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        KeyboardTimestampBridge::default(),
        Vector2::new(160.0, 28.0),
    );

    assert_eq!(
        runner.route_native_key_release(PhysicalKey::Code(WinitKeyCode::Numpad1)),
        None
    );
    assert!(runner.core.runtime.bridge().messages.is_empty());

    assert!(
        !runner
            .route_native_key_release(PhysicalKey::Code(WinitKeyCode::ArrowDown))
            .expect("supported physical release should produce a route outcome")
            .routed
    );
    assert!(runner.core.runtime.bridge().messages.is_empty());
}

#[test]
fn focused_text_input_enter_and_tab_preserve_native_key_metadata() {
    let enter_timestamp = Some(InputTimestamp::capture());
    let tab_timestamp = Some(InputTimestamp::capture());
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        KeyboardTimestampBridge::default(),
        Vector2::new(160.0, 28.0),
    );

    assert!(runner.core.runtime.focus_widget(90));
    runner.input.modifiers = ModifiersState::SHIFT;

    let mut enter_outcome = GenericRouteOutcome::default();
    assert!(runner.route_focused_text_input_before_shortcuts(
        KeyCode::Enter,
        None,
        enter_timestamp,
        true,
        &mut enter_outcome,
    ));

    let mut tab_outcome = GenericRouteOutcome::default();
    assert!(runner.route_focused_text_input_before_shortcuts(
        KeyCode::Tab,
        None,
        tab_timestamp,
        false,
        &mut tab_outcome,
    ));

    assert_eq!(
        runner.core.runtime.bridge().messages,
        vec![
            KeyboardTimestampMessage::KeyPress {
                modifiers: KeyboardModifiers {
                    command: false,
                    control: false,
                    shift: true,
                    alt: false,
                },
                repeat: true,
                timestamp: enter_timestamp,
            },
            KeyboardTimestampMessage::KeyPress {
                modifiers: KeyboardModifiers {
                    command: false,
                    control: false,
                    shift: true,
                    alt: false,
                },
                repeat: false,
                timestamp: tab_timestamp,
            },
        ]
    );
}

#[test]
fn printable_text_fanout_reuses_one_timestamp_for_every_character() {
    let timestamp = Some(InputTimestamp::capture());
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        KeyboardTimestampBridge::default(),
        Vector2::new(160.0, 28.0),
    );
    assert!(runner.core.runtime.focus_widget(90));

    let mut outcome = GenericRouteOutcome::default();
    assert!(runner.route_text_input_after_unhandled_keypress("éx", timestamp, &mut outcome));
    assert_eq!(
        runner.core.runtime.bridge().messages,
        vec![
            KeyboardTimestampMessage::Character {
                character: 'é',
                timestamp,
            },
            KeyboardTimestampMessage::Character {
                character: 'x',
                timestamp,
            },
        ]
    );
}

#[test]
fn direct_text_edit_route_preserves_timestamp() {
    let timestamp = Some(InputTimestamp::capture());
    let mut core = GenericNativeRuntimeCore::new(
        KeyboardTimestampBridge::default(),
        Vector2::new(160.0, 28.0),
    );
    assert!(core.runtime.focus_widget(90));

    // The canvas accepts the normalized command directly; the native text-input
    // eligibility gate is covered by the focused text-input routing tests.
    assert!(
        core.runtime
            .dispatch_focused_input(WidgetInput::TextEdit {
                command: TextEditCommand::SelectAll,
                timestamp,
            })
            .is_some()
    );
    assert_eq!(
        core.runtime.bridge().messages,
        vec![KeyboardTimestampMessage::TextEdit(timestamp)]
    );
}
