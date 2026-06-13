use super::*;

#[test]
fn runtime_controller_groups_work_interaction_and_traversal_state() {
    let controller = controller_source("src/runtime/controller.rs");
    let work = controller_source("src/runtime/controller/work.rs");
    let interaction_state = controller_source("src/runtime/controller/interaction_state.rs");
    let traversal_state = controller_source("src/runtime/controller/traversal_state.rs");

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
}

#[test]
fn runtime_controller_context_and_scratch_modules_use_explicit_dependencies() {
    let context = controller_source("src/runtime/controller/context.rs");
    let context_frame = controller_source("src/runtime/controller/context/frame.rs");
    let scratch = controller_source("src/runtime/controller/scratch.rs");
    let focus = controller_source("src/runtime/controller/focus.rs");

    assert!(
        context.contains("use super::SurfaceRuntime;")
            && context.contains("gui::types::{Rect, Vector2}")
            && context.contains("layout::{LayoutDebugOptions, LayoutOutput, NodeId}")
            && context.contains("runtime::{RuntimeBridge, RuntimeDiagnostics, UiSurface}")
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
}

#[test]
fn runtime_controller_state_modules_use_explicit_dependencies() {
    let state = controller_source("src/runtime/controller/state.rs");
    let state_layout = controller_source("src/runtime/controller/state/layout.rs");
    let state_lifecycle = controller_source("src/runtime/controller/state/lifecycle.rs");
    let state_traversal = controller_source("src/runtime/controller/state/traversal.rs");

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
            && state_lifecycle.contains("CommandOutcome")
            && state_lifecycle.contains("RuntimeBridge")
            && state_lifecycle.contains("SurfaceRuntimeProjection")
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
}
