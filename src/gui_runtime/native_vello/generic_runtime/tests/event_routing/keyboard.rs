use super::super::*;
use crate::{
    gui::input::{InputTimestamp, KeyCode, KeyPress},
    runtime::{RuntimeBridge, SurfaceNode, UiSurface},
    widgets::{CanvasMessage, TextEditCommand, WidgetInput, WidgetKey, WidgetSizing},
};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
enum KeyboardTimestampMessage {
    KeyPress(Option<InputTimestamp>),
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
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::canvas_mapped(
            90,
            WidgetSizing::fixed(Vector2::new(160.0, 28.0)),
            |message| match message {
                CanvasMessage::Input {
                    input: WidgetInput::KeyPress { timestamp, .. },
                } => KeyboardTimestampMessage::KeyPress(timestamp),
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
            },
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
        )
        .routed
    );
    assert_eq!(
        core.runtime.bridge().messages,
        vec![KeyboardTimestampMessage::KeyPress(timestamp)]
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
