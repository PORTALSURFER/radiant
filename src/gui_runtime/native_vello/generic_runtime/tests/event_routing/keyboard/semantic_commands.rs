use super::*;
use crate::application::{CommandInput, CommandKey, ShortcutPlatform};

#[test]
fn semantic_commands_never_steal_a_captured_focused_key_sequence() {
    let mut core = GenericNativeRuntimeCore::new(
        FocusedKeyRoutingBridge::new(false, false),
        Vector2::new(160.0, 28.0),
    );
    assert!(core.runtime.focus_widget(91));
    let mut input =
        CommandInput::logical(CommandKey::Named("ArrowUp".into()), ShortcutPlatform::Mac);
    let press = KeyPress {
        key: KeyCode::ArrowUp,
        command: false,
        control: false,
        shift: false,
        alt: false,
    };
    assert!(
        core.route_metadata_command_key_press(
            Some(press),
            Some(WidgetKey::ArrowUp),
            KeyboardModifiers::default(),
            None,
            &input
        )
        .unwrap()
        .routed
    );
    assert_eq!(core.runtime.bridge().semantic_presses, 1);
    assert_eq!(core.runtime.bridge().messages.len(), 1);
    core.runtime.bridge_mut().semantic_handled = true;
    input.repeat = true;
    core.route_metadata_command_key_press(
        Some(press),
        Some(WidgetKey::ArrowUp),
        KeyboardModifiers::default(),
        None,
        &input,
    );
    assert_eq!(core.runtime.bridge().semantic_presses, 1);
    assert_eq!(core.runtime.bridge().messages.len(), 2);
    input.repeat = false;
    core.route_metadata_command_key_press(
        Some(press),
        Some(WidgetKey::ArrowRight),
        KeyboardModifiers::default(),
        None,
        &input,
    );
    assert_eq!(core.runtime.bridge().semantic_presses, 1);
    assert_eq!(core.runtime.bridge().messages.len(), 2);
    core.route_key_release_with_metadata(WidgetKey::ArrowUp, KeyboardModifiers::default(), None);
    assert_eq!(core.runtime.bridge().messages.len(), 3);
    input.repeat = false;
    core.route_metadata_command_key_press(
        Some(press),
        Some(WidgetKey::ArrowUp),
        KeyboardModifiers::default(),
        None,
        &input,
    );
    // The existing post-refresh ownership fence consumes one ambiguous sample.
    assert_eq!(core.runtime.bridge().semantic_presses, 1);
    core.route_metadata_command_key_press(
        Some(press),
        Some(WidgetKey::ArrowUp),
        KeyboardModifiers::default(),
        None,
        &input,
    );
    assert_eq!(core.runtime.bridge().semantic_presses, 2);
    assert_eq!(core.runtime.bridge().messages.len(), 3);
    assert_eq!(core.runtime.bridge().host_presses.len(), 1);
}

#[test]
fn text_owned_metadata_key_reaches_the_widget_without_a_host_shortcut_pass() {
    let mut bridge = FocusedKeyRoutingBridge::new(true, false);
    bridge.semantic_handled = true;
    let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(160.0, 28.0));
    assert!(core.runtime.focus_widget(91));
    let mut input =
        CommandInput::logical(CommandKey::Named("ArrowLeft".into()), ShortcutPlatform::Mac);
    input.text_consumed = true;
    let press = KeyPress {
        key: KeyCode::ArrowLeft,
        command: false,
        control: false,
        shift: false,
        alt: false,
    };
    core.route_metadata_command_key_press(
        Some(press),
        Some(WidgetKey::ArrowLeft),
        KeyboardModifiers::default(),
        None,
        &input,
    );
    assert_eq!(core.runtime.bridge().messages.len(), 1);
    assert_eq!(core.runtime.bridge().semantic_presses, 1);
    assert_eq!(core.runtime.bridge().host_presses.len(), 0);
}
