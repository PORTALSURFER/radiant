use super::{DemoMessage, intrinsic_slot};
use radiant::{
    layout::{Point, Vector2},
    runtime::{
        Command, RuntimeBridge, ScrollFixedRowIntoViewParts, ScrollIntoViewParts, ScrollUpdate,
        SurfaceChild, SurfaceNode, SurfaceRuntime, UiSurface, declarative_runtime_bridge,
    },
    widgets::WidgetSizing,
};
use std::sync::Arc;

#[path = "runtime/scrollbar_affordance.rs"]
mod scrollbar_affordance;

struct ScrollObserverBridge {
    surface: Arc<UiSurface<DemoMessage>>,
    updates: usize,
    last_update: Option<ScrollUpdate>,
}

impl RuntimeBridge<DemoMessage> for ScrollObserverBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        Arc::clone(&self.surface)
    }

    fn scroll_updated(&mut self, update: ScrollUpdate) -> Option<Command<DemoMessage>> {
        self.updates += 1;
        self.last_update = Some(update);
        None
    }
}

#[test]
fn surface_runtime_skips_scroll_update_when_clamped_offset_is_unchanged() {
    let surface = Arc::new(UiSurface::<DemoMessage>::new(SurfaceNode::scroll_area(
        31,
        SurfaceNode::column(
            32,
            2.0,
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

    assert!(runtime.scroll_at(Point::new(20.0, 20.0), Vector2::new(0.0, 0.0)));

    assert_eq!(runtime.bridge().updates, 0);
    assert!(
        !runtime.take_repaint_requested(),
        "unchanged scroll offsets should not notify the host or request repaint"
    );
}

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
fn surface_runtime_scroll_fixed_row_into_view_does_not_drift_over_repeated_navigation() {
    const ROW_HEIGHT: f32 = 24.0;
    const ROWS: u64 = 40;
    const VISIBLE_ROWS: usize = 10;
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

#[test]
fn surface_runtime_routes_scroll_delta_to_scroll_view_under_pointer() {
    let bridge = declarative_runtime_bridge(
        Arc::new(UiSurface::<DemoMessage>::new(SurfaceNode::scroll_area(
            31,
            SurfaceNode::column(
                32,
                2.0,
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
        ))),
        |surface| Arc::clone(surface),
        |_, _message| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 96.0));
    let before = runtime.layout().rects[&100];

    assert!(runtime.scroll_at(Point::new(20.0, 20.0), Vector2::new(0.0, 48.0)));
    let after = runtime.layout().rects[&100];

    assert!(after.min.y < before.min.y);
    assert_eq!(before.min.y - after.min.y, 48.0);
}

#[test]
fn surface_runtime_does_not_hit_scrolled_content_outside_scroll_viewport() {
    let bridge = declarative_runtime_bridge(
        Arc::new(UiSurface::<DemoMessage>::new(SurfaceNode::column(
            1,
            8.0,
            vec![
                SurfaceChild::new(
                    intrinsic_slot(),
                    SurfaceNode::button(
                        10,
                        "Header",
                        WidgetSizing::fixed(Vector2::new(220.0, 32.0)),
                        DemoMessage::Increment,
                    ),
                ),
                SurfaceChild::fill(SurfaceNode::scroll_area(
                    20,
                    SurfaceNode::column(
                        21,
                        2.0,
                        (0..12)
                            .map(|index| {
                                SurfaceChild::new(
                                    intrinsic_slot(),
                                    SurfaceNode::button(
                                        100 + index,
                                        format!("Row {index}"),
                                        WidgetSizing::fixed(Vector2::new(220.0, 30.0)),
                                        DemoMessage::Increment,
                                    ),
                                )
                            })
                            .collect(),
                    ),
                )),
            ],
        ))),
        |surface| Arc::clone(surface),
        |_, _message| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 120.0));

    assert!(runtime.scroll_at(Point::new(20.0, 60.0), Vector2::new(0.0, 80.0)));

    assert_eq!(runtime.widget_at(Point::new(20.0, 16.0)), Some(10));
}

#[test]
fn surface_runtime_does_not_scroll_nested_view_outside_parent_clip() {
    let bridge = declarative_runtime_bridge(
        Arc::new(UiSurface::<DemoMessage>::new(SurfaceNode::scroll_area(
            20,
            SurfaceNode::column(
                21,
                0.0,
                vec![
                    SurfaceChild::new(
                        intrinsic_slot(),
                        SurfaceNode::text(
                            30,
                            "Spacer",
                            WidgetSizing::fixed(Vector2::new(220.0, 100.0)),
                        ),
                    ),
                    SurfaceChild::new(
                        intrinsic_slot(),
                        SurfaceNode::scroll_area(
                            40,
                            SurfaceNode::column(
                                41,
                                0.0,
                                (0..8)
                                    .map(|index| {
                                        SurfaceChild::new(
                                            intrinsic_slot(),
                                            SurfaceNode::text(
                                                100 + index,
                                                format!("Nested {index}"),
                                                WidgetSizing::fixed(Vector2::new(220.0, 24.0)),
                                            ),
                                        )
                                    })
                                    .collect(),
                            ),
                        ),
                    ),
                ],
            ),
        ))),
        |surface| Arc::clone(surface),
        |_, _message| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 80.0));

    assert!(!runtime.scroll_at(Point::new(20.0, 110.0), Vector2::new(0.0, 24.0)));
}
