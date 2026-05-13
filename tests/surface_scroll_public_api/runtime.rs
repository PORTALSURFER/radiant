use super::{DemoMessage, intrinsic_slot};
use radiant::{
    layout::{Point, Vector2},
    runtime::{
        Command, Event, PaintPrimitive, RuntimeBridge, ScrollUpdate, SurfaceChild, SurfaceNode,
        SurfaceRuntime, UiSurface, declarative_runtime_bridge,
    },
    widgets::{PointerButton, WidgetSizing},
};
use std::sync::Arc;

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
fn surface_runtime_drags_painted_scrollbar_thumb() {
    let bridge = declarative_runtime_bridge(
        Arc::new(UiSurface::<DemoMessage>::new(SurfaceNode::scroll_area(
            31,
            SurfaceNode::column(
                32,
                2.0,
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
        ))),
        |surface| Arc::clone(surface),
        |_, _message| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 96.0));
    let before = runtime.layout().rects[&100];
    let thumb = runtime
        .paint_plan(&Default::default())
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) if fill.widget_id == 31 => Some(fill.rect),
            _ => None,
        })
        .expect("scroll area should paint a draggable thumb");

    runtime.dispatch_event(Event::PointerPress {
        position: thumb.center(),
        button: PointerButton::Primary,
    });
    assert_eq!(runtime.hovered_scroll_affordance(), Some(31));
    assert!(
        runtime.take_repaint_requested(),
        "pressing the painted scroll thumb should request a redraw"
    );
    runtime.dispatch_event(Event::PointerMove {
        position: Point::new(thumb.center().x, thumb.center().y + 36.0),
    });
    assert!(
        runtime.take_repaint_requested(),
        "dragging the painted scroll thumb should request a redraw"
    );
    runtime.dispatch_event(Event::PointerRelease {
        position: Point::new(thumb.center().x, thumb.center().y + 36.0),
        button: PointerButton::Primary,
    });

    let after = runtime.layout().rects[&100];
    assert!(after.min.y < before.min.y);
}

#[test]
fn surface_runtime_highlights_painted_scrollbar_thumb_on_hover() {
    let bridge = declarative_runtime_bridge(
        Arc::new(UiSurface::<DemoMessage>::new(SurfaceNode::scroll_area(
            31,
            SurfaceNode::column(
                32,
                2.0,
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
        ))),
        |surface| Arc::clone(surface),
        |_, _message| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 96.0));
    let theme = radiant::theme::ThemeTokens::default();
    let thumb = runtime
        .paint_plan(&theme)
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) if fill.widget_id == 31 => Some(fill.rect),
            _ => None,
        })
        .expect("scroll area should paint a hoverable thumb");

    runtime.dispatch_event(Event::PointerMove {
        position: thumb.center(),
    });

    let hovered_color = runtime
        .paint_plan(&theme)
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) if fill.widget_id == 31 => Some(fill.color),
            _ => None,
        })
        .expect("hovered scroll area should still paint a thumb");
    assert_eq!(runtime.hovered_scroll_affordance(), Some(31));
    assert!(runtime.take_repaint_requested());
    assert_eq!(hovered_color, theme.accent_copper);

    runtime.dispatch_event(Event::PointerMove {
        position: Point::new(8.0, 8.0),
    });
    let idle_color = runtime
        .paint_plan(&theme)
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) if fill.widget_id == 31 => Some(fill.color),
            _ => None,
        })
        .expect("idle scroll area should still paint a thumb");
    assert_eq!(runtime.hovered_scroll_affordance(), None);
    assert!(runtime.take_repaint_requested());
    assert_eq!(idle_color, theme.grid_strong);
}

#[test]
fn surface_runtime_clears_scrollbar_hover_when_refresh_removes_scroll_area() {
    let bridge = declarative_runtime_bridge(
        0_u8,
        |state| {
            let node = if *state == 0 {
                SurfaceNode::scroll_area(
                    31,
                    SurfaceNode::column(
                        32,
                        2.0,
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
                )
            } else {
                SurfaceNode::text(
                    40,
                    "No scroll",
                    WidgetSizing::fixed(Vector2::new(180.0, 24.0)),
                )
            };
            Arc::new(UiSurface::<DemoMessage>::new(node))
        },
        |state, message| match message {
            DemoMessage::Increment => *state = state.saturating_add(1),
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 96.0));
    let thumb = runtime
        .paint_plan(&Default::default())
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) if fill.widget_id == 31 => Some(fill.rect),
            _ => None,
        })
        .expect("scroll area should paint a hoverable thumb");

    runtime.dispatch_event(Event::PointerMove {
        position: thumb.center(),
    });
    assert_eq!(runtime.hovered_scroll_affordance(), Some(31));

    runtime.dispatch_message(DemoMessage::Increment);

    assert_eq!(runtime.hovered_scroll_affordance(), None);
    assert!(
        !runtime.layout().rects.contains_key(&31),
        "the refreshed layout should no longer contain the hovered scroll area"
    );
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
