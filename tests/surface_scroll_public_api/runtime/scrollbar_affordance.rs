use super::*;
use radiant::runtime::{Event, PaintPrimitive};
use radiant::widgets::PointerButton;

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
