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
    let drag_drop_hover = fs::read_to_string(
        manifest_dir.join(
            "src/gui_runtime/native_vello/generic_runtime/tests/event_routing/drag_drop/hover_routing.rs",
        ),
    )
    .expect("native drag/drop hover-routing tests should be readable");
    let drag_drop_fixtures = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/tests/event_routing/drag_drop/fixtures.rs",
    ))
    .expect("native drag/drop fixtures should be readable");

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
            && drag_drop.contains("mod hover_routing;")
            && drag_drop.contains("mod fixtures;")
            && drag_drop_fixtures.contains("struct DropBridge")
            && drag_drop_hover
                .contains("fn captured_drag_routes_pointer_move_to_hovered_drop_target"),
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
fn native_pointer_click_classification_stays_in_focused_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let runtime_root =
        fs::read_to_string(manifest_dir.join("src/gui_runtime/native_vello/generic_runtime.rs"))
            .expect("native runtime root should be readable");
    let event_routing = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/event_routing.rs"),
    )
    .expect("native event routing module should be readable");
    let pointer_click = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/pointer_click.rs"),
    )
    .expect("native pointer click module should be readable");

    assert!(
        runtime_root.contains("mod pointer_click;")
            && runtime_root.contains("use pointer_click::pointer_press_event;"),
        "native runtime root should expose pointer click classification through a focused helper"
    );
    assert!(
        event_routing.contains("pointer_press_event(")
            && !event_routing.contains("DOUBLE_CLICK_MAX_INTERVAL")
            && !event_routing.contains("DOUBLE_CLICK_MAX_DISTANCE")
            && !event_routing.contains("fn is_double_click("),
        "native event routing should delegate double-click classification instead of owning timing and distance policy"
    );
    assert!(
        pointer_click.contains("use super::PointerPressStamp;")
            && pointer_click.contains("runtime::Event")
            && pointer_click.contains("widgets::{PointerButton, PointerModifiers}")
            && pointer_click.contains("const DOUBLE_CLICK_MAX_INTERVAL")
            && pointer_click.contains("const DOUBLE_CLICK_MAX_DISTANCE")
            && pointer_click.contains("fn pointer_press_event(")
            && pointer_click.contains("fn is_double_click(")
            && pointer_click.contains("nearby_repeated_press_routes_as_double_click")
            && pointer_click
                .contains("stale_distant_or_different_button_press_routes_as_single_press")
            && !pointer_click.starts_with("use super::*;"),
        "native pointer click classification should keep pure double-click policy, event selection, and tests together"
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
        lifecycle_pointer.contains("GenericNativeVelloRunner")
            && lifecycle_pointer.contains("GenericRouteOutcome")
            && lifecycle_pointer.contains("logical_point_from_winit")
            && lifecycle_pointer.contains("maybe_log_route_profile")
            && lifecycle_pointer.contains("use crate::runtime::RuntimeBridge;")
            && lifecycle_pointer.contains("use std::time::Instant;")
            && lifecycle_pointer
                .contains("use winit::{dpi::PhysicalPosition, event_loop::ActiveEventLoop")
            && !lifecycle_pointer.starts_with("use super::*;"),
        "native pointer lifecycle should name runner, input conversion, profiling, bridge, timing, and winit dependencies"
    );
    assert!(
        lifecycle_pointer.contains("fn handle_cursor_moved(")
            && lifecycle_pointer.contains("fn handle_cursor_left(")
            && lifecycle_pointer.contains("PhysicalPosition<f64>")
            && lifecycle_pointer
                .contains("logical_point_from_winit(position, self.window.dpi_scale)")
            && lifecycle_pointer.contains("maybe_log_route_profile(\"pointer_move\"")
            && !lifecycle_pointer.contains("winit::dpi::PhysicalPosition"),
        "native pointer lifecycle should keep cursor move and cursor-left routing focused"
    );
}
