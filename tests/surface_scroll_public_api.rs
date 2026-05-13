//! Public API coverage for scroll and virtual-scroll surface behavior.

use radiant::{
    layout::{
        Constraints, LayoutDebugOptions, LayoutState, Point, Rect, SizeModeCross, SizeModeMain,
        SlotParams, Vector2, VirtualizationAxis, layout_tree, layout_tree_with_state,
    },
    runtime::{SurfaceChild, SurfaceNode, UiSurface},
    widgets::WidgetSizing,
};

#[path = "surface_scroll_public_api/paint.rs"]
mod paint;
#[path = "surface_scroll_public_api/runtime.rs"]
mod runtime;

#[derive(Clone, Debug, PartialEq)]
enum DemoMessage {
    Increment,
}

fn intrinsic_slot() -> SlotParams {
    SlotParams {
        size_main: SizeModeMain::Intrinsic,
        size_cross: SizeModeCross::Fill,
        constraints: Constraints::unconstrained(),
        margin: Default::default(),
        align_cross_override: None,
        allow_fixed_compress: false,
    }
}

#[test]
fn surface_node_scroll_area_helpers_project_scroll_view_layout() {
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::scroll_area(
        31,
        SurfaceNode::text(
            32,
            "Long content",
            WidgetSizing::fixed(Vector2::new(220.0, 160.0)),
        ),
    ));

    let output = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0)),
    );
    let overflow = output
        .overflow_flags
        .get(&31)
        .expect("scroll area should report overflow");

    assert!(surface.find_widget(32).is_some());
    assert!(overflow.x);
    assert!(overflow.y);
}

#[test]
fn surface_node_virtual_scroll_area_helper_records_virtual_window() {
    let rows = (0..256)
        .map(|index| {
            SurfaceChild::new(
                intrinsic_slot(),
                SurfaceNode::text(
                    1000 + index,
                    format!("Row {index}"),
                    WidgetSizing::fixed(Vector2::new(180.0, 10.0)),
                ),
            )
        })
        .collect::<Vec<_>>();
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::virtual_scroll_area(
        33,
        SurfaceNode::column(34, 1.0, rows),
        VirtualizationAxis::Vertical,
        0.0,
    ));
    let mut state = LayoutState::default();
    state.scroll_offsets.insert(33, Vector2::new(0.0, 400.0));

    let output = layout_tree_with_state(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(220.0, 120.0)),
        &state,
        LayoutDebugOptions::default(),
    );
    let window = output
        .virtual_windows
        .get(&33)
        .expect("virtual scroll area should report a virtual window");

    assert_eq!(window.total_children, 256);
    assert!(window.first_index > 0);
    assert!(window.culled_after > 0);
}
