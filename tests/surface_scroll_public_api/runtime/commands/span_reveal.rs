use super::*;

#[test]
fn surface_runtime_scroll_into_view_uses_actual_viewport_height_and_margins() {
    let surface = Arc::new(UiSurface::<DemoMessage>::new(SurfaceNode::scroll_area(
        31,
        SurfaceNode::column(
            32,
            0.0,
            (0..12)
                .map(|index| {
                    SurfaceChild::new(
                        intrinsic_slot(),
                        SurfaceNode::text(
                            100 + index as u64,
                            format!("Row {index}"),
                            WidgetSizing::fixed(Vector2::new(180.0, 24.0)),
                        ),
                    )
                })
                .collect(),
        ),
    )));
    let bridge = ScrollObserverBridge {
        surface,
        updates: 0,
        last_update: None,
    };
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 96.0));

    let outcome = runtime.execute_command(Command::scroll_into_view(
        31,
        5.0 * 24.0,
        24.0,
        2.0 * 24.0,
        2.0 * 24.0,
    ));

    assert!(outcome.repaint_requested);
    let update = runtime
        .bridge()
        .last_update
        .expect("scroll into view should report clamped runtime offset");
    assert_eq!(update.offset.y, 96.0);
    assert_eq!(update.viewport.y, 96.0);
}

#[test]
fn surface_runtime_scroll_into_view_can_snap_to_fixed_rows() {
    let surface = Arc::new(UiSurface::<DemoMessage>::new(SurfaceNode::scroll_area(
        31,
        SurfaceNode::column(
            32,
            0.0,
            (0..12)
                .map(|index| {
                    SurfaceChild::new(
                        intrinsic_slot(),
                        SurfaceNode::text(
                            100 + index,
                            format!("Row {index}"),
                            WidgetSizing::fixed(Vector2::new(180.0, 24.0)),
                        ),
                    )
                })
                .collect(),
        ),
    )));
    let bridge = ScrollObserverBridge {
        surface,
        updates: 0,
        last_update: None,
    };
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 95.0));

    runtime.execute_command(Command::scroll_into_view_snapped(
        31,
        5.0 * 24.0,
        24.0,
        2.0 * 24.0,
        2.0 * 24.0,
        24.0,
    ));

    let update = runtime
        .bridge()
        .last_update
        .expect("snapped scroll into view should move the scroll container");
    assert_eq!(update.offset.y, 96.0);
}

#[test]
fn surface_runtime_scroll_commands_support_named_parts_construction() {
    let surface = Arc::new(UiSurface::<DemoMessage>::new(SurfaceNode::scroll_area(
        31,
        SurfaceNode::column(
            32,
            0.0,
            (0..20)
                .map(|index| {
                    SurfaceChild::new(
                        intrinsic_slot(),
                        SurfaceNode::text(
                            100 + index,
                            format!("Row {index}"),
                            WidgetSizing::fixed(Vector2::new(180.0, 24.0)),
                        ),
                    )
                })
                .collect(),
        ),
    )));
    let bridge = ScrollObserverBridge {
        surface,
        updates: 0,
        last_update: None,
    };
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 96.0));

    runtime.execute_command(Command::scroll_into_view_from_parts(ScrollIntoViewParts {
        node_id: 31,
        target_y: 5.0 * 24.0,
        target_height: 24.0,
        margin_top: 2.0 * 24.0,
        margin_bottom: 2.0 * 24.0,
        snap_y: Some(24.0),
    }));

    let update = runtime
        .bridge()
        .last_update
        .expect("named-parts span reveal should move the scroll container");
    assert_eq!(update.offset.y, 96.0);

    runtime.bridge_mut().last_update = None;
    runtime.execute_command(Command::scroll_fixed_row_into_view_from_parts(
        ScrollFixedRowIntoViewParts {
            node_id: 31,
            row_index: 8,
            row_stride: 24.0,
            leading_context_rows: 2,
            trailing_context_rows: 2,
            direction: 1,
        },
    ));

    let update = runtime
        .bridge()
        .last_update
        .expect("named-parts fixed-row reveal should move the scroll container");
    assert_eq!(update.offset.y, 168.0);
}
