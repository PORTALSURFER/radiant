use super::super::*;
use crate::{
    gui::{
        input::{InputTimestamp, KeyCode, KeyPress},
        types::Rect,
    },
    layout::LayoutOutput,
    runtime::{PaintPrimitive, RuntimeBridge, SurfaceNode, UiSurface, WidgetMessageMapper},
    theme::ThemeTokens,
    widgets::{
        CanvasMessage, CanvasWidget, KeyboardModifiers, TextEditCommand, Widget, WidgetCommon,
        WidgetId, WidgetInput, WidgetKey, WidgetOutput, WidgetSizing,
    },
};
use std::sync::Arc;
use winit::keyboard::ModifiersState;

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
