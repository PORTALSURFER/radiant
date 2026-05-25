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
    let context = fs::read_to_string(manifest_dir.join("src/runtime/controller/context.rs"))
        .expect("runtime controller context module should be readable");
    let context_frame =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/context/frame.rs"))
            .expect("runtime controller context frame module should be readable");
    let scratch = fs::read_to_string(manifest_dir.join("src/runtime/controller/scratch.rs"))
        .expect("runtime controller scratch buffers should be readable");
    let focus = fs::read_to_string(manifest_dir.join("src/runtime/controller/focus.rs"))
        .expect("runtime controller focus module should be readable");
    let state = fs::read_to_string(manifest_dir.join("src/runtime/controller/state.rs"))
        .expect("runtime controller state module should be readable");
    let state_layout =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/state/layout.rs"))
            .expect("runtime controller state layout module should be readable");
    let state_lifecycle =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/state/lifecycle.rs"))
            .expect("runtime controller state lifecycle module should be readable");
    let state_traversal =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/state/traversal.rs"))
            .expect("runtime controller state traversal module should be readable");
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
    let scroll_wheel =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/scroll/wheel.rs"))
            .expect("runtime controller scroll wheel module should be readable");
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
        context.contains("use super::SurfaceRuntime;")
            && context.contains("gui::types::{Rect, Vector2}")
            && context.contains("layout::{LayoutDebugOptions, LayoutOutput, NodeId}")
            && context.contains("runtime::{RuntimeBridge, UiSurface}")
            && context.contains("widgets::WidgetId")
            && !context.starts_with("use super::*;")
            && context.contains("pub struct RuntimeContext<'a, Message>"),
        "runtime controller context should name view, layout, geometry, bridge, and widget dependencies without inheriting the controller root"
    );
    assert!(
        context_frame.contains("use super::super::SurfaceRuntime;")
            && context_frame.contains("gui::types::{Point, Rect}")
            && context_frame.contains("layout::LayoutOutput")
            && context_frame.contains("PaintPrimitive")
            && context_frame.contains("RuntimeBridge")
            && context_frame.contains("SurfaceFrame")
            && context_frame.contains("SurfacePaintPlan")
            && context_frame.contains("empty_paint_plan_for_layout")
            && context_frame.contains("theme::ThemeTokens")
            && context_frame.contains("widgets::WidgetId")
            && !context_frame.starts_with("use super::super::*;")
            && context_frame.contains("fn append_widget_runtime_overlay")
            && context_frame.contains("fn append_drag_preview_overlay"),
        "runtime controller context frame helpers should name paint, layout, geometry, theme, bridge, and widget dependencies without inheriting the controller root"
    );
    assert!(
        scratch.contains("use crate::{")
            && scratch.contains("gui::types::Vector2")
            && scratch.contains("layout::NodeId")
            && !scratch.starts_with("use super::*;")
            && scratch.contains("pub(super) struct RuntimeScratch")
            && scratch.contains("scroll_clamp_updates: Vec<(NodeId, Vector2)>")
            && scratch.contains("projection_scroll_stack: Vec<NodeId>")
            && scratch.contains("projection_child_path: Vec<usize>"),
        "runtime controller scratch buffers should name layout and geometry dependencies without inheriting the controller root"
    );
    assert!(
        focus.contains("use super::{FocusTraversal, SurfaceRuntime};")
            && focus.contains("gui::{focus::FocusSurface, input::KeyPress}")
            && focus.contains("runtime::RuntimeBridge")
            && focus.contains("widgets::{WidgetId, WidgetInput, WidgetKey}")
            && !focus.starts_with("use super::*;")
            && focus.contains("fn next_focus_target"),
        "runtime controller focus routing should name traversal, keyboard, bridge, and widget dependencies without inheriting the controller root"
    );
    assert!(
        state.contains("use super::SurfaceRuntime;")
            && state.contains("use crate::{runtime::RuntimeBridge, widgets::WidgetId};")
            && !state.contains("use super::*;")
            && state.contains("mod layout;")
            && state.contains("mod lifecycle;")
            && state.contains("mod traversal;")
            && state.contains("fn capture_pointer_capture_state")
            && state.contains("fn restore_pointer_capture_state"),
        "runtime controller state root should name controller, bridge, and widget identity dependencies without inheriting the controller root"
    );
    assert!(
        state_layout.contains("use super::super::{SurfaceRuntime, SurfaceTraversalIndex};")
            && state_layout.contains("gui::types::Vector2")
            && state_layout.contains("layout::LayoutDiagnosticCode")
            && state_layout.contains("runtime::RuntimeBridge")
            && !state_layout.starts_with("use super::super::*;")
            && state_layout.contains("fn sync_scroll_offsets"),
        "runtime controller state layout should name controller, traversal, geometry, diagnostic, and bridge dependencies without inheriting the controller root"
    );
    assert!(
        state_lifecycle.contains("RuntimeInteractionState")
            && state_lifecycle.contains("RuntimeScratch")
            && state_lifecycle.contains("RuntimeTraversalState")
            && state_lifecycle.contains("RuntimeWorkQueues")
            && state_lifecycle.contains("SurfaceRuntime")
            && state_lifecycle.contains("gui::types::{Point, Rect, Vector2}")
            && state_lifecycle
                .contains("layout::{LayoutDebugOptions, LayoutEngine, LayoutOutput, LayoutState}")
            && state_lifecycle.contains("runtime::{RuntimeBridge, SurfaceRuntimeProjection}")
            && !state_lifecycle.starts_with("use super::super::*;")
            && state_lifecycle.contains("fn clear_stale_interaction_state")
            && state_lifecycle.contains("fn normalized_viewport"),
        "runtime controller state lifecycle should name runtime state, geometry, layout, bridge, and projection dependencies without inheriting the controller root"
    );
    assert!(
        state_traversal.contains("use super::super::SurfaceRuntime;")
            && state_traversal.contains("runtime::{RuntimeBridge, SurfaceTraversalIndex}")
            && !state_traversal.starts_with("use super::super::*;")
            && state_traversal.contains("fn install_traversal_index")
            && state_traversal.contains("fn take_reusable_traversal_index"),
        "runtime controller state traversal should name controller, bridge, and traversal dependencies without inheriting the controller root"
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
        scroll_wheel.contains("use super::super::{CommandOutcome, SurfaceRuntime};")
            && scroll_wheel.contains("gui::types::{Point, Vector2}")
            && scroll_wheel.contains("runtime::{RuntimeBridge, WidgetDispatchResult}")
            && scroll_wheel.contains("widgets::{PointerModifiers, WidgetId, WidgetInput}")
            && !scroll_wheel.starts_with("use super::super::*;")
            && scroll_wheel.contains("fn dispatch_wheel_at_with_refresh")
            && scroll_wheel.contains("fn wheel_widget_at"),
        "runtime controller wheel routing should name command outcome, controller, geometry, bridge, dispatch result, pointer, and widget dependencies without inheriting the controller root"
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
    let pointer_events =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/events/pointer.rs"))
            .expect("runtime pointer event controller should be readable");
    let pointer = fs::read_to_string(manifest_dir.join("src/runtime/controller/pointer.rs"))
        .expect("runtime pointer controller should be readable");
    let move_routing =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/pointer/move_routing.rs"))
            .expect("runtime pointer move routing module should be readable");
    let hit_test = fs::read_to_string(manifest_dir.join("src/runtime/controller/hit_test.rs"))
        .expect("runtime controller hit-test module should be readable");
    let hit_order = fs::read_to_string(manifest_dir.join("src/runtime/controller/hit_order.rs"))
        .expect("runtime controller hit-order module should be readable");
    let input = fs::read_to_string(manifest_dir.join("src/runtime/controller/input.rs"))
        .expect("runtime controller input module should be readable");

    assert!(
        pointer_events.contains("use super::SurfaceRuntime;")
            && pointer_events.contains("gui::types::Point")
            && pointer_events.contains("runtime::RuntimeBridge")
            && pointer_events
                .contains("widgets::{PointerButton, PointerModifiers, WidgetId, WidgetInput}")
            && !pointer_events.starts_with("use super::*;")
            && pointer_events.contains("fn dispatch_pointer_press_event")
            && pointer_events.contains("fn dispatch_pointer_double_click_event")
            && pointer_events.contains("fn dispatch_pointer_release_event"),
        "pointer event routing should name controller, geometry, bridge, and widget-input dependencies without inheriting the event root"
    );
    assert!(
        pointer.contains("mod move_routing;")
            && pointer.contains("use super::{PointerMoveOutcome, SurfaceRuntime};")
            && pointer.contains("gui::types::Point")
            && pointer.contains("runtime::RuntimeBridge")
            && pointer.contains("widgets::{WidgetId, WidgetInput}")
            && !pointer.starts_with("use super::*;")
            && !pointer.contains("fn route_pointer_move_to_target")
            && !pointer.contains("fn update_drag_preview_position"),
        "pointer controller root should name pointer, geometry, bridge, and widget dependencies while delegating pointer-move routing internals"
    );
    assert!(
        move_routing.contains("fn dispatch_pointer_move_target")
            && move_routing.contains("use super::{PointerMoveDispatch, SurfaceRuntime};")
            && move_routing.contains("gui::types::Point")
            && move_routing.contains("runtime::RuntimeBridge")
            && move_routing.contains("widgets::{WidgetId, WidgetInput}")
            && !move_routing.starts_with("use super::*;")
            && move_routing.contains("fn route_pointer_move_to_target")
            && move_routing.contains("fn update_drag_preview_position")
            && move_routing.contains("fn update_hovered_scroll_affordance")
            && move_routing.contains("fn route_captured_pass_through_move"),
        "pointer move routing should group drag preview, hover, and captured move policy while naming its controller, geometry, bridge, and widget dependencies"
    );
    assert!(
        hit_test.contains("use super::SurfaceRuntime;")
            && hit_test.contains("gui::types::Point")
            && hit_test.contains("layout::NodeId")
            && hit_test.contains("runtime::{RuntimeBridge, SurfaceWidget}")
            && hit_test.contains("widgets::WidgetId")
            && !hit_test.starts_with("use super::*;")
            && hit_test.contains("pub fn widget_at")
            && hit_test.contains("fn stable_hovered_widget_at"),
        "runtime controller hit testing should name controller, geometry, layout, bridge, widget, and traversal dependencies"
    );
    assert!(
        hit_order.contains("use std::collections::HashMap;")
            && hit_order.contains("use crate::layout::{LayoutOutput, NodeId};")
            && !hit_order.starts_with("use super::*;")
            && hit_order.contains("pub(super) struct HitOrderIndex")
            && hit_order.contains("fn collect_hit_rank")
            && hit_order.contains("fn collect_visible_hit_order"),
        "runtime controller hit-order indexing should name its collection, layout, and node dependencies without inheriting the controller root"
    );
    assert!(
        input.contains("use super::SurfaceRuntime;")
            && input.contains("gui::types::Rect")
            && input.contains("runtime::{RuntimeBridge, SurfaceWidget, WidgetDispatchResult}")
            && input.contains("widgets::{WidgetId, WidgetInput}")
            && !input.starts_with("use super::*;")
            && input.contains("fn dispatch_surface_input")
            && input.contains("fn surface_widget_mut"),
        "runtime controller input dispatch should name controller, geometry, bridge, widget, dispatch-result, and widget-input dependencies"
    );
}
