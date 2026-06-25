use super::*;

#[test]
fn surface_runtime_scroll_fixed_row_into_view_anchors_directionally() {
    let surface = Arc::new(UiSurface::<DemoMessage>::new(SurfaceNode::scroll_area(
        31,
        SurfaceNode::column(
            32,
            0.0,
            (0..30)
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
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 10.0 * 24.0));

    runtime.execute_command(Command::scroll_fixed_row_into_view(31, 8, 24.0, 2, 2, 1));
    let update = runtime
        .bridge()
        .last_update
        .expect("downward fixed-row reveal should keep two rows below the target");
    assert_eq!(update.offset.y, 24.0);

    runtime.execute_command(Command::scroll_to(31, Vector2::new(0.0, 10.0 * 24.0)));
    runtime.bridge_mut().last_update = None;
    runtime.execute_command(Command::scroll_fixed_row_into_view(31, 13, 24.0, 2, 2, -1));
    assert!(
        runtime.bridge().last_update.is_none(),
        "upward navigation should not scroll while more than two rows remain above the target"
    );

    runtime.execute_command(Command::scroll_fixed_row_into_view(31, 11, 24.0, 2, 2, -1));
    let update = runtime
        .bridge()
        .last_update
        .expect("upward fixed-row reveal should keep two rows above the target");
    assert_eq!(update.offset.y, 9.0 * 24.0);
}

#[test]
fn surface_runtime_scroll_fixed_row_into_view_counts_partial_trailing_row() {
    const ROW_HEIGHT: f32 = 24.0;
    let surface = Arc::new(UiSurface::<DemoMessage>::new(SurfaceNode::scroll_area(
        31,
        SurfaceNode::column(
            32,
            0.0,
            (0..30)
                .map(|index| {
                    SurfaceChild::new(
                        intrinsic_slot(),
                        SurfaceNode::text(
                            100 + index,
                            format!("Row {index}"),
                            WidgetSizing::fixed(Vector2::new(180.0, ROW_HEIGHT)),
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
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, ROW_HEIGHT * 10.5));

    runtime.execute_command(Command::scroll_fixed_row_into_view(
        31, 8, ROW_HEIGHT, 2, 2, 1,
    ));

    assert!(
        runtime.bridge().last_update.is_none(),
        "downward fixed-row reveal should treat the partially visible trailing row as visible"
    );
}

#[test]
fn surface_runtime_scroll_fixed_row_into_view_neutral_direction_reveals_above_and_below() {
    const ROW_HEIGHT: f32 = 24.0;
    let surface = Arc::new(UiSurface::<DemoMessage>::new(SurfaceNode::scroll_area(
        31,
        SurfaceNode::column(
            32,
            0.0,
            (0..30)
                .map(|index| {
                    SurfaceChild::new(
                        intrinsic_slot(),
                        SurfaceNode::text(
                            100 + index,
                            format!("Row {index}"),
                            WidgetSizing::fixed(Vector2::new(180.0, ROW_HEIGHT)),
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
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, ROW_HEIGHT * 10.0));

    runtime.execute_command(Command::scroll_to(31, Vector2::new(0.0, ROW_HEIGHT * 10.0)));
    runtime.bridge_mut().last_update = None;
    runtime.execute_command(Command::scroll_fixed_row_into_view(
        31, 5, ROW_HEIGHT, 2, 2, 0,
    ));
    let update = runtime
        .bridge()
        .last_update
        .expect("neutral fixed-row reveal should scroll upward for rows above the viewport");
    assert_eq!(update.offset.y, ROW_HEIGHT * 3.0);

    runtime.execute_command(Command::scroll_to(31, Vector2::new(0.0, 0.0)));
    runtime.bridge_mut().last_update = None;
    runtime.execute_command(Command::scroll_fixed_row_into_view(
        31, 14, ROW_HEIGHT, 2, 2, 0,
    ));
    let update = runtime
        .bridge()
        .last_update
        .expect("neutral fixed-row reveal should scroll downward for rows below the viewport");
    assert_eq!(update.offset.y, ROW_HEIGHT * 7.0);

    runtime.bridge_mut().last_update = None;
    runtime.execute_command(Command::scroll_fixed_row_into_view(
        31, 13, ROW_HEIGHT, 2, 2, 0,
    ));
    assert!(
        runtime.bridge().last_update.is_none(),
        "neutral fixed-row reveal should keep a row visible with enough context"
    );
}

#[test]
fn surface_runtime_scroll_fixed_row_into_view_does_not_drift_over_repeated_navigation() {
    const ROW_HEIGHT: f32 = 24.0;
    const ROWS: u64 = 40;
    const VISIBLE_ROWS: usize = 11;
    let viewport_height = ROW_HEIGHT * 10.5;
    let max_offset = ROWS as f32 * ROW_HEIGHT - viewport_height;
    let surface = Arc::new(UiSurface::<DemoMessage>::new(SurfaceNode::scroll_area(
        31,
        SurfaceNode::column(
            32,
            0.0,
            (0..ROWS)
                .map(|index| {
                    SurfaceChild::new(
                        intrinsic_slot(),
                        SurfaceNode::text(
                            100 + index,
                            format!("Row {index}"),
                            WidgetSizing::fixed(Vector2::new(180.0, ROW_HEIGHT)),
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
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, viewport_height));
    let mut offset_y = 0.0;

    for row_index in 0..ROWS as usize {
        runtime.bridge_mut().last_update = None;
        runtime.execute_command(Command::scroll_fixed_row_into_view(
            31, row_index, ROW_HEIGHT, 2, 2, 1,
        ));
        if let Some(update) = runtime.bridge().last_update {
            offset_y = update.offset.y;
        }
        let expected = row_index.saturating_add(3).saturating_sub(VISIBLE_ROWS) as f32 * ROW_HEIGHT;
        assert_eq!(offset_y, expected.min(max_offset), "down row {row_index}");
    }

    for row_index in (0..ROWS as usize).rev() {
        runtime.bridge_mut().last_update = None;
        runtime.execute_command(Command::scroll_fixed_row_into_view(
            31, row_index, ROW_HEIGHT, 2, 2, -1,
        ));
        let top_limit = row_index.saturating_sub(2) as f32 * ROW_HEIGHT;
        if offset_y > top_limit {
            offset_y = top_limit;
        }
        if let Some(update) = runtime.bridge().last_update {
            assert_eq!(update.offset.y, offset_y, "up row {row_index}");
        }
    }
}

#[test]
fn surface_runtime_scroll_fixed_row_into_view_uses_row_stride() {
    let surface = Arc::new(UiSurface::<DemoMessage>::new(SurfaceNode::scroll_area(
        31,
        SurfaceNode::column(
            32,
            4.0,
            (0..30)
                .map(|index| {
                    SurfaceChild::new(
                        intrinsic_slot(),
                        SurfaceNode::text(
                            100 + index,
                            format!("Row {index}"),
                            WidgetSizing::fixed(Vector2::new(180.0, 20.0)),
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
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 10.0 * 24.0));

    runtime.execute_command(Command::scroll_fixed_row_into_view(31, 8, 24.0, 2, 2, 1));

    let update = runtime
        .bridge()
        .last_update
        .expect("fixed-row reveal should accept row stride separately from visual row height");
    assert_eq!(update.offset.y, 24.0);
}
