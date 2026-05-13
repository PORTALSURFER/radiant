use super::{DemoMessage, intrinsic_slot};
use radiant::{
    layout::{Point, Vector2},
    runtime::{
        Command, RuntimeBridge, ScrollUpdate, SurfaceChild, SurfaceNode, SurfaceRuntime, UiSurface,
        declarative_runtime_bridge,
    },
    widgets::WidgetSizing,
};
use std::sync::Arc;

#[path = "runtime/scrollbar_affordance.rs"]
mod scrollbar_affordance;

struct ScrollObserverBridge {
    surface: Arc<UiSurface<DemoMessage>>,
    updates: usize,
}

impl RuntimeBridge<DemoMessage> for ScrollObserverBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        Arc::clone(&self.surface)
    }

    fn scroll_updated(&mut self, _update: ScrollUpdate) -> Option<Command<DemoMessage>> {
        self.updates += 1;
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
