use super::*;

#[test]
fn native_event_routing_tests_stay_grouped_by_input_concern() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/tests/event_routing.rs"),
    )
    .expect("native event-routing test root should be readable");
    let host = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/tests/event_routing/host.rs"),
    )
    .expect("native host event-routing tests should be readable");
    let canvas = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/tests/event_routing/canvas.rs"),
    )
    .expect("native canvas event-routing tests should be readable");
    let scroll = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/tests/event_routing/scroll.rs"),
    )
    .expect("native scroll event-routing tests should be readable");
    let repaint = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/tests/event_routing/repaint.rs"),
    )
    .expect("native repaint event-routing tests should be readable");
    let drag_drop = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/tests/event_routing/drag_drop.rs"),
    )
    .expect("native drag/drop event-routing tests should be readable");

    assert!(
        root.contains("mod host;")
            && root.contains("mod canvas;")
            && root.contains("mod scroll;")
            && root.contains("mod repaint;")
            && root.contains("mod drag_drop;")
            && !root.contains("fn generic_core_routes_pointer_and_key_input")
            && !root.contains("struct DropBridge"),
        "native event-routing test root should index focused input groups instead of owning all event cases"
    );
    assert!(
        host.contains("fn generic_core_routes_text_edit_commands_only_to_text_inputs")
            && canvas.contains("fn generic_canvas_can_receive_keyboard_focus_and_text_input")
            && scroll
                .contains("fn scrollbar_drag_state_survives_view_refresh_after_offset_message")
            && repaint.contains("fn generic_core_drains_command_repaint_requests_after_routing")
            && drag_drop.contains("struct DropBridge")
            && drag_drop.contains("fn captured_drag_routes_pointer_move_to_hovered_drop_target"),
        "native event-routing tests should stay grouped by host, canvas, scroll, repaint, and drag/drop concerns"
    );
}

#[test]
fn native_file_drop_routing_uses_explicit_runtime_imports() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let native_file_drop = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/native_file_drop.rs"),
    )
    .expect("native file drop routing module should be readable");

    assert!(
        native_file_drop.contains("use super::GenericNativeVelloRunner;")
            && native_file_drop.contains("use crate::runtime::{NativeFileDrop, RuntimeBridge};")
            && native_file_drop.contains("use crate::widgets::WidgetId;")
            && native_file_drop.contains("use std::path::PathBuf;")
            && native_file_drop.contains("use winit::event_loop::ActiveEventLoop;")
            && !native_file_drop.starts_with("use super::*;"),
        "native file drop routing should name its runner, runtime model, widget id, path, and event-loop dependencies"
    );
    assert!(
        native_file_drop.contains("NativeFileDrop::hover")
            && native_file_drop.contains("NativeFileDrop::cancel")
            && native_file_drop.contains("NativeFileDrop::dropped")
            && native_file_drop.contains("native_file_drop_target"),
        "native file drop routing should keep hover, cancel, drop, and target resolution in the focused module"
    );
}

#[test]
fn native_keyboard_repeat_policy_uses_explicit_imports() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repeat = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/keyboard/repeat.rs"),
    )
    .expect("native keyboard repeat policy module should be readable");

    assert!(
        repeat.contains("use crate::gui::input::KeyCode;")
            && repeat.contains("use std::time::{Duration, Instant};")
            && !repeat.starts_with("use super::*;"),
        "native keyboard repeat policy should import only key and timing dependencies"
    );
    assert!(
        repeat.contains("const NAVIGATION_KEY_REPEAT_INTERVAL")
            && repeat.contains("fn should_route_keypress(")
            && repeat.contains("KeyCode::ArrowUp | KeyCode::ArrowDown")
            && !repeat.contains("crate::gui::input::KeyCode::"),
        "native keyboard repeat policy should stay focused on navigation key throttling"
    );
}

#[test]
fn native_keyboard_routing_uses_explicit_imports() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let keyboard = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/keyboard.rs"),
    )
    .expect("native keyboard routing module should be readable");

    assert!(
        keyboard.contains("use super::{")
            && keyboard.contains("GenericNativeVelloRunner")
            && keyboard.contains("GenericRouteOutcome")
            && keyboard.contains("key_code_from_winit")
            && keyboard.contains("keypress_from_input")
            && keyboard.contains("use crate::{runtime::RuntimeBridge, widgets::WidgetKey};")
            && keyboard.contains("use std::time::Instant;")
            && keyboard.contains("event::{ElementState, KeyEvent}")
            && keyboard.contains("event_loop::ActiveEventLoop")
            && keyboard.contains("keyboard::{Key, NamedKey, PhysicalKey}")
            && !keyboard.starts_with("use super::*;"),
        "native keyboard routing should name runner, route outcome, input conversion, widget key, bridge, timing, and winit dependencies"
    );
    assert!(
        keyboard.contains("fn handle_keyboard_event(")
            && keyboard.contains("event: KeyEvent")
            && keyboard.contains("should_route_keypress(")
            && keyboard.contains("route_text_input_shortcut")
            && keyboard.contains("WidgetKey::from_key_code")
            && !keyboard.contains("winit::event::KeyEvent"),
        "native keyboard routing should keep physical-key, shortcut, text, and widget-key paths focused"
    );
}

#[test]
fn native_keyboard_text_edit_routing_uses_explicit_imports() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let text_edit = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/keyboard/text_edit.rs"),
    )
    .expect("native keyboard text-edit routing module should be readable");

    assert!(
        text_edit.contains("use super::{GenericNativeVelloRunner, GenericRouteOutcome};")
            && text_edit.contains("use crate::gui::input::KeyCode;")
            && text_edit.contains("use crate::runtime::RuntimeBridge;")
            && text_edit.contains("use crate::widgets::TextEditCommand;")
            && !text_edit.starts_with("use super::*;"),
        "native keyboard text-edit routing should name runner, route outcome, key, bridge, and command dependencies"
    );
    assert!(
        text_edit.contains("fn route_space_text_input(")
            && text_edit.contains("fn route_text_input_shortcut(")
            && text_edit.contains("fn route_text_navigation_key(")
            && text_edit.contains("fn route_text_input(")
            && text_edit.contains("KeyCode::ArrowLeft")
            && text_edit.contains("TextEditCommand::MoveWordLeft")
            && !text_edit.contains("crate::gui::input::KeyCode::"),
        "native keyboard text-edit routing should keep shortcut, navigation, space, and printable text paths focused"
    );
}

#[test]
fn native_pointer_lifecycle_uses_explicit_imports() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lifecycle_pointer = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/lifecycle_pointer.rs"),
    )
    .expect("native pointer lifecycle module should be readable");

    assert!(
        lifecycle_pointer.contains(
            "use super::{GenericNativeVelloRunner, logical_point_from_winit, maybe_log_route_profile};"
        )
            && lifecycle_pointer.contains("use crate::runtime::RuntimeBridge;")
            && lifecycle_pointer.contains("use std::time::Instant;")
            && lifecycle_pointer
                .contains("use winit::{dpi::PhysicalPosition, event_loop::ActiveEventLoop};")
            && !lifecycle_pointer.starts_with("use super::*;"),
        "native pointer lifecycle should name runner, input conversion, profiling, bridge, timing, and winit dependencies"
    );
    assert!(
        lifecycle_pointer.contains("fn handle_cursor_moved(")
            && lifecycle_pointer.contains("fn handle_cursor_left(")
            && lifecycle_pointer.contains("PhysicalPosition<f64>")
            && lifecycle_pointer.contains("logical_point_from_winit(position)")
            && lifecycle_pointer.contains("maybe_log_route_profile(\"pointer_move\"")
            && !lifecycle_pointer.contains("winit::dpi::PhysicalPosition"),
        "native pointer lifecycle should keep cursor move and cursor-left routing focused"
    );
}
