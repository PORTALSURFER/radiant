use super::*;

#[test]
fn controller_commands_keep_outcome_drain_and_dispatch_in_focused_modules() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let controller = fs::read_to_string(manifest_dir.join("src/runtime/controller.rs"))
        .expect("runtime controller root should be readable");
    let interaction_state =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/interaction_state.rs"))
            .expect("runtime controller interaction state should be readable");
    let traversal_state =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/traversal_state.rs"))
            .expect("runtime controller traversal state should be readable");
    let root = fs::read_to_string(manifest_dir.join("src/runtime/controller/commands.rs"))
        .expect("runtime controller command root should be readable");
    let work = fs::read_to_string(manifest_dir.join("src/runtime/controller/work.rs"))
        .expect("runtime controller work queues should be readable");
    let outcome =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/outcome.rs"))
            .expect("runtime command outcome module should be readable");
    let drain = fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/drain.rs"))
        .expect("runtime command drain module should be readable");
    let dispatch =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/dispatch.rs"))
            .expect("runtime command dispatch module should be readable");
    let external_drag =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/external_drag.rs"))
            .expect("runtime command external drag module should be readable");
    let scrolling =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/scrolling.rs"))
            .expect("runtime command scrolling module should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/tests.rs"))
        .expect("runtime command test root should be readable");
    let test_batching =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/tests/batching.rs"))
            .expect("runtime command batching tests should be readable");
    let test_drain =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/tests/drain.rs"))
            .expect("runtime command drain tests should be readable");
    let test_external_drag = fs::read_to_string(
        manifest_dir.join("src/runtime/controller/commands/tests/external_drag.rs"),
    )
    .expect("runtime command external drag tests should be readable");
    let test_platform =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/tests/platform.rs"))
            .expect("runtime command platform tests should be readable");
    let test_fixtures =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/tests/fixtures.rs"))
            .expect("runtime command test fixtures should be readable");

    for required in [
        "mod dispatch;",
        "mod drain;",
        "mod outcome;",
        "pub use outcome::CommandOutcome;",
    ] {
        assert!(
            root.contains(required),
            "runtime controller command root should delegate `{required}`"
        );
    }
    assert!(
        root.contains("use super::SurfaceRuntime;")
            && root.contains("use crate::runtime::{Command, RuntimeBridge};")
            && root.contains("#[cfg(test)]")
            && root.contains("gui::types::{Point, Vector2}")
            && root.contains("runtime::UiSurface")
            && !root.starts_with("use super::*;"),
        "runtime controller command root should name production dependencies and keep fixture-only geometry/surface imports under cfg(test)"
    );
    assert!(
        outcome.contains("pub struct CommandOutcome")
            && outcome.contains("fn finish_command_outcome")
            && outcome.contains("use super::SurfaceRuntime;")
            && outcome.contains("use crate::runtime::RuntimeBridge;")
            && !outcome.starts_with("use super::*;")
            && !root.contains("pub struct CommandOutcome"),
        "command pass result and finalization should live in commands/outcome.rs with explicit controller and bridge dependencies"
    );
    assert!(
        drain.contains("pub fn drain_runtime_messages")
            && drain.contains(".drain_bridge_commands")
            && drain.contains(".drain_bridge_messages")
            && !root.contains("pub fn drain_runtime_messages"),
        "runtime work draining should live in commands/drain.rs"
    );
    assert!(
        controller.contains("mod work;")
            && controller.contains("runtime_work: RuntimeWorkQueues<Message>")
            && !controller.contains("runtime_commands: Vec<Command<Message>>")
            && !controller.contains("runtime_messages: Vec<Message>"),
        "surface runtime should keep runtime work queues behind one focused controller field"
    );
    assert!(
        work.contains("pub(super) struct RuntimeWorkQueues<Message>")
            && work.contains("fn drain_bridge_commands")
            && work.contains("fn drain_bridge_messages")
            && work.contains("fn has_remaining_work"),
        "runtime work queue ownership should live in controller/work.rs"
    );
    assert!(
        controller.contains("mod interaction_state;")
            && controller.contains("interaction: RuntimeInteractionState<Message>")
            && !controller.contains("focused_widget: Option<WidgetId>")
            && !controller.contains("pointer_capture: Option<WidgetId>")
            && !controller.contains("drag_session: Option<DragSession>"),
        "surface runtime should group interaction ownership behind one focused controller field"
    );
    assert!(
        interaction_state.contains("pub(super) struct RuntimeInteractionState<Message>")
            && interaction_state.contains("pub(super) struct RuntimeFocusState")
            && interaction_state.contains("pub(super) struct RuntimeHoverState")
            && interaction_state.contains("pub(super) struct RuntimePointerState")
            && interaction_state.contains("pub(super) struct RuntimeDragState<Message>"),
        "focus, hover, pointer capture, and drag state should stay in controller/interaction_state.rs"
    );
    assert!(
        controller.contains("mod traversal_state;")
            && controller.contains("traversal: RuntimeTraversalState")
            && !controller.contains("widget_hit_order: Vec<WidgetId>")
            && !controller.contains("focusable_widgets: HitOrderIndex")
            && !controller.contains("widget_paths: HashMap"),
        "surface runtime should group projected traversal indexes behind one focused controller field"
    );
    assert!(
        traversal_state.contains("pub(super) struct RuntimeTraversalState")
            && traversal_state.contains("pub(super) struct RuntimeWidgetTraversal")
            && traversal_state.contains("pub(super) struct RuntimeWidgetPathState")
            && traversal_state.contains("pub(super) struct RuntimeContainerTraversal"),
        "widget paths, hit orders, and container indexes should stay in controller/traversal_state.rs"
    );
    assert!(
        dispatch.contains("fn execute_command_inner")
            && dispatch.contains("Command::PlatformRequest")
            && dispatch.contains("Command::ScrollFixedRowIntoView")
            && dispatch.contains("use super::{CommandOutcome, SurfaceRuntime};")
            && dispatch.contains("gui::types::Vector2")
            && dispatch
                .contains("runtime::{Command, DragSession, ExternalDragSession, RuntimeBridge}")
            && !dispatch.starts_with("use super::*;")
            && !root.contains("fn execute_command_inner"),
        "command execution branches should live in commands/dispatch.rs"
    );
    assert!(
        external_drag.contains("use super::{CommandOutcome, SurfaceRuntime};")
            && external_drag
                .contains("runtime::{ExternalDragOutcome, ExternalDragSession, RuntimeBridge}")
            && !external_drag.starts_with("use super::*;")
            && scrolling.contains("use super::super::{ScrollUpdate, SurfaceRuntime};")
            && scrolling.contains("gui::types::{Point, Vector2}")
            && scrolling.contains("layout::NodeId")
            && scrolling.contains("runtime::RuntimeBridge")
            && !scrolling.starts_with("use super::super::*;"),
        "external drag and scrolling command helpers should own their drag, scroll, geometry, layout, and bridge dependencies"
    );
    assert!(
        tests.contains("mod batching;")
            && tests.contains("mod drain;")
            && tests.contains("mod external_drag;")
            && tests.contains("mod platform;")
            && tests.contains("mod fixtures;")
            && !tests.contains("fn runtime_command_batch_preserves_order_and_keeps_remainder"),
        "runtime controller command test root should index focused behavior groups instead of owning all cases"
    );
    assert!(
        test_batching.contains("fn runtime_command_batch_preserves_order_and_keeps_remainder")
            && test_drain
                .contains("fn runtime_command_drains_are_bounded_and_request_followup_wakeup")
            && test_external_drag
                .contains("fn external_drag_command_arms_and_clears_native_session")
            && test_platform.contains("fn platform_request_dispatches_through_bridge_completion")
            && test_fixtures.contains("struct QueuedCommandBridge"),
        "runtime controller command tests should stay grouped by batching, drain, external drag, platform, and fixtures concerns"
    );
}

#[test]
fn pointer_controller_keeps_move_routing_in_focused_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pointer = fs::read_to_string(manifest_dir.join("src/runtime/controller/pointer.rs"))
        .expect("runtime pointer controller should be readable");
    let move_routing =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/pointer/move_routing.rs"))
            .expect("runtime pointer move routing module should be readable");

    assert!(
        pointer.contains("mod move_routing;")
            && !pointer.contains("fn route_pointer_move_to_target")
            && !pointer.contains("fn update_drag_preview_position"),
        "pointer controller root should delegate pointer-move routing internals"
    );
    assert!(
        move_routing.contains("fn dispatch_pointer_move_target")
            && move_routing.contains("fn route_pointer_move_to_target")
            && move_routing.contains("fn update_drag_preview_position")
            && move_routing.contains("fn update_hovered_scroll_affordance")
            && move_routing.contains("fn route_captured_pass_through_move"),
        "pointer move routing should group drag preview, hover, and captured move policy"
    );
}
