use crate::{
    gui::input::InputTimestamp,
    gui::types::Rect,
    layout::LayoutOutput,
    runtime::{
        Event, PaintPrimitive, RuntimeBridge, SurfaceNode, SurfaceRuntime, UiSurface,
        WidgetMessageMapper,
    },
    theme::ThemeTokens,
    widgets::{
        KeyboardModifiers, TextEditCommand, Widget, WidgetCommon, WidgetInput, WidgetKey,
        WidgetOutput, WidgetSizing,
    },
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq)]
enum KeyboardTimestampMessage {
    KeyPress(Option<InputTimestamp>),
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
}

#[derive(Clone)]
struct KeyboardTimestampWidget {
    common: WidgetCommon,
}

impl KeyboardTimestampWidget {
    fn new() -> Self {
        Self {
            common: WidgetCommon::new(
                40,
                WidgetSizing::fixed(crate::gui::types::Vector2::new(120.0, 40.0)),
            )
            .with_keyboard_focus(),
        }
    }
}

impl Widget for KeyboardTimestampWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::KeyPress { timestamp, .. } => Some(WidgetOutput::typed(
                KeyboardTimestampMessage::KeyPress(timestamp),
            )),
            WidgetInput::KeyRelease {
                key,
                modifiers,
                timestamp,
            } => Some(WidgetOutput::typed(KeyboardTimestampMessage::KeyRelease {
                key,
                modifiers,
                timestamp,
            })),
            WidgetInput::Character {
                character,
                timestamp,
            } => Some(WidgetOutput::typed(KeyboardTimestampMessage::Character {
                character,
                timestamp,
            })),
            WidgetInput::TextEdit { timestamp, .. } => Some(WidgetOutput::typed(
                KeyboardTimestampMessage::TextEdit(timestamp),
            )),
            _ => None,
        }
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}

#[derive(Default)]
struct KeyboardTimestampBridge {
    messages: Vec<KeyboardTimestampMessage>,
}

impl RuntimeBridge<KeyboardTimestampMessage> for KeyboardTimestampBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<KeyboardTimestampMessage>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
            KeyboardTimestampWidget::new(),
            WidgetMessageMapper::typed(|message: KeyboardTimestampMessage| message),
        )))
    }

    fn reduce_message(&mut self, message: KeyboardTimestampMessage) {
        self.messages.push(message);
    }
}

#[test]
fn injected_keyboard_event_timestamp_survives_event_to_widget_dispatch() {
    let timestamp = Some(InputTimestamp::capture());
    let mut runtime = SurfaceRuntime::new(
        KeyboardTimestampBridge::default(),
        crate::gui::types::Vector2::new(120.0, 40.0),
    );

    assert!(runtime.focus_widget(40));
    assert_eq!(
        runtime.dispatch_event(Event::KeyPress {
            key: WidgetKey::Enter,
            modifiers: KeyboardModifiers::default(),
            repeat: false,
            timestamp,
        }),
        Some(40)
    );
    assert_eq!(
        runtime.dispatch_event(Event::Character {
            character: 'x',
            timestamp,
        }),
        Some(40)
    );
    let release_modifiers = KeyboardModifiers {
        command: true,
        control: true,
        shift: false,
        alt: true,
    };
    assert_eq!(
        runtime.dispatch_event(Event::KeyRelease {
            key: WidgetKey::ArrowDown,
            modifiers: release_modifiers,
            timestamp,
        }),
        Some(40)
    );
    assert_eq!(
        runtime.bridge().messages,
        vec![
            KeyboardTimestampMessage::KeyPress(timestamp),
            KeyboardTimestampMessage::Character {
                character: 'x',
                timestamp,
            },
            KeyboardTimestampMessage::KeyRelease {
                key: WidgetKey::ArrowDown,
                modifiers: release_modifiers,
                timestamp,
            },
        ]
    );
}

#[test]
fn key_release_event_without_focus_is_not_routed() {
    let mut runtime = SurfaceRuntime::new(
        KeyboardTimestampBridge::default(),
        crate::gui::types::Vector2::new(120.0, 40.0),
    );

    assert_eq!(
        runtime.dispatch_event(Event::key_release(WidgetKey::ArrowDown)),
        None
    );
    assert!(runtime.bridge().messages.is_empty());
}

#[test]
fn direct_text_edit_timestamp_survives_widget_dispatch() {
    let timestamp = Some(InputTimestamp::capture());
    let mut runtime = SurfaceRuntime::new(
        KeyboardTimestampBridge::default(),
        crate::gui::types::Vector2::new(120.0, 40.0),
    );

    assert!(runtime.focus_widget(40));
    assert_eq!(
        runtime.dispatch_focused_input(WidgetInput::TextEdit {
            command: TextEditCommand::SelectAll,
            timestamp,
        }),
        Some(40)
    );
    assert_eq!(
        runtime.bridge().messages,
        vec![KeyboardTimestampMessage::TextEdit(timestamp)]
    );
}
