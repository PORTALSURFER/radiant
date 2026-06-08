use super::*;

#[test]
fn pointer_controller_keeps_move_routing_in_focused_module() {
    let pointer_events = controller_source("src/runtime/controller/events/pointer.rs");
    let pointer = controller_source("src/runtime/controller/pointer.rs");
    let move_routing = controller_source("src/runtime/controller/pointer/move_routing.rs");
    let hit_test = controller_source("src/runtime/controller/hit_test.rs");
    let hit_order = controller_source("src/runtime/controller/hit_order.rs");
    let input = controller_source("src/runtime/controller/input.rs");

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
            && pointer.contains("runtime::{CommandOutcome, NativeFileDrop, RuntimeBridge}")
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
