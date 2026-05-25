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
