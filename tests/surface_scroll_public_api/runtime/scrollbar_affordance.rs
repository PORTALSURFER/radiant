use super::*;
use radiant::widgets::PointerButton;
use radiant::{
    layout::{ContainerKind, ContainerPolicy, OverflowPolicy, ScrollPolicy, ScrollbarVisibility},
    runtime::{Event, PaintPrimitive},
    widgets::{PointerModifiers, WheelDelta, WheelPhase, WheelSample},
};
use std::{cell::Cell, rc::Rc, sync::Arc};

fn visibility_scroll_surface(visibility: ScrollbarVisibility) -> UiSurface<DemoMessage> {
    let scroll = SurfaceNode::container(
        31,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            scroll_policy: ScrollPolicy::default().scrollbar_visibility(visibility),
            ..ContainerPolicy::default()
        },
        vec![SurfaceChild::fill(SurfaceNode::text(
            32,
            "Long content",
            WidgetSizing::fixed(Vector2::new(180.0, 400.0)),
        ))],
    );
    UiSurface::new(SurfaceNode::stack(
        1,
        vec![
            SurfaceChild::fill(SurfaceNode::button(
                40,
                "Underlying",
                WidgetSizing::fixed(Vector2::new(220.0, 96.0)),
                DemoMessage::Increment,
            )),
            SurfaceChild::fill(scroll),
        ],
    ))
}

#[test]
#[allow(clippy::arc_with_non_send_sync)]
fn auto_scrollbar_capture_requires_the_same_visibility_as_paint() {
    let always = SurfaceRuntime::new(
        declarative_runtime_bridge(
            (),
            move |_| Arc::new(visibility_scroll_surface(ScrollbarVisibility::Always)),
            |_, _| {},
        ),
        Vector2::new(220.0, 96.0),
    );
    let thumb = always
        .paint_plan(&Default::default())
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) if fill.widget_id == 31 => Some(fill.rect),
            _ => None,
        })
        .expect("Always should paint the latent thumb geometry");

    let mut auto = SurfaceRuntime::new(
        declarative_runtime_bridge(
            (),
            move |_| Arc::new(visibility_scroll_surface(ScrollbarVisibility::Auto)),
            |_, _| {},
        ),
        Vector2::new(220.0, 96.0),
    );
    assert!(
        !auto.paint_plan(&Default::default()).primitives.iter().any(
            |primitive| matches!(primitive, PaintPrimitive::FillRect(fill) if fill.widget_id == 31)
        ),
        "idle Auto must not paint a thumb"
    );
    assert!(auto.focus_widget(40));
    assert_eq!(auto.focused_widget(), Some(40));
    let before = auto.layout().rects[&32];
    auto.dispatch_event(Event::primary_press(thumb.center()));
    auto.dispatch_event(Event::pointer_move(Point::new(
        thumb.center().x,
        thumb.center().y + 36.0,
    )));
    auto.dispatch_event(Event::primary_release(Point::new(
        thumb.center().x,
        thumb.center().y + 36.0,
    )));
    assert_eq!(auto.layout().rects[&32], before);
    assert_eq!(auto.focused_widget(), Some(40));
    assert_eq!(auto.hovered_scroll_affordance(), None);

    auto.dispatch_event(Event::pointer_move(thumb.center()));
    assert!(
        auto.paint_plan(&Default::default()).primitives.iter().any(
            |primitive| matches!(primitive, PaintPrimitive::FillRect(fill) if fill.widget_id == 31)
        ),
        "hovered Auto should paint the thumb"
    );
    auto.dispatch_event(Event::primary_press(thumb.center()));
    auto.dispatch_event(Event::pointer_move(Point::new(
        thumb.center().x,
        thumb.center().y + 36.0,
    )));
    auto.dispatch_event(Event::primary_release(Point::new(
        thumb.center().x,
        thumb.center().y + 36.0,
    )));
    assert!(auto.layout().rects[&32].min.y < before.min.y);
}

#[test]
#[allow(clippy::arc_with_non_send_sync)]
fn phase_less_settlement_is_finalized_before_a_new_explicit_scroll_sequence() {
    let settled_a = Rc::new(Cell::new(0_usize));
    let settled_b = Rc::new(Cell::new(0_usize));
    let callback_a = Rc::clone(&settled_a);
    let callback_b = Rc::clone(&settled_b);
    let mut runtime = SurfaceRuntime::new(
        declarative_runtime_bridge(
            (Rc::clone(&settled_a), Rc::clone(&settled_b)),
            move |_| {
                let a = Rc::clone(&callback_a);
                let b = Rc::clone(&callback_b);
                Arc::new(UiSurface::<DemoMessage>::new(SurfaceNode::column(
                    1,
                    0.0,
                    vec![
                        SurfaceChild::fill(
                            SurfaceNode::scroll_area(
                                31,
                                SurfaceNode::text(
                                    32,
                                    "Long content",
                                    WidgetSizing::fixed(Vector2::new(180.0, 400.0)),
                                ),
                            )
                            .on_offset_settled(move |_| {
                                a.set(a.get() + 1);
                                DemoMessage::ScrollSettled
                            }),
                        ),
                        SurfaceChild::fill(
                            SurfaceNode::scroll_area(
                                41,
                                SurfaceNode::text(
                                    42,
                                    "Long content",
                                    WidgetSizing::fixed(Vector2::new(180.0, 400.0)),
                                ),
                            )
                            .on_offset_settled(move |_| {
                                b.set(b.get() + 1);
                                DemoMessage::ScrollSettled
                            }),
                        ),
                    ],
                )))
            },
            |_, message| {
                if message == DemoMessage::ScrollSettled {
                    // The callbacks above record the owning container.
                }
            },
        ),
        Vector2::new(220.0, 96.0),
    );
    let point_a = Point::new(10.0, 10.0);
    let point_b = Point::new(10.0, 70.0);
    let delta = WheelDelta::Pixels(Vector2::new(0.0, 8.0));
    let phase_less = WheelSample::phase_less(delta, PointerModifiers::default())
        .expect("finite phase-less wheel sample");
    assert!(runtime.wheel_or_scroll_at_with_sample(point_a, phase_less));
    assert_eq!(settled_a.get(), 0);
    assert_eq!(settled_b.get(), 0);

    let started = WheelSample::new(
        delta,
        Some(WheelPhase::Started),
        PointerModifiers::default(),
    )
    .expect("finite started wheel sample");
    assert!(runtime.wheel_or_scroll_at_with_sample(point_b, started));
    assert_eq!(
        settled_a.get(),
        1,
        "owner A settles at the replacement Start"
    );
    assert_eq!(settled_b.get(), 0, "owner B must not settle at Start");

    let changed = WheelSample::new(
        delta,
        Some(WheelPhase::Changed),
        PointerModifiers::default(),
    )
    .expect("finite changed wheel sample");
    assert!(runtime.wheel_or_scroll_at_with_sample(point_b, changed));
    assert_eq!(settled_b.get(), 0, "owner B must not settle before End");
    let ended = WheelSample::new(delta, Some(WheelPhase::Ended), PointerModifiers::default())
        .expect("finite ended wheel sample");
    assert!(runtime.wheel_or_scroll_at_with_sample(point_b, ended));
    assert_eq!(settled_a.get(), 1);
    assert_eq!(settled_b.get(), 1, "owner B settles once at End");

    assert_eq!(
        settled_a.get(),
        1,
        "the old phase-less deadline cannot settle owner A again"
    );
}

#[test]
fn surface_runtime_drags_painted_scrollbar_thumb() {
    let bridge = declarative_runtime_bridge(
        crate::arc_surface(UiSurface::<DemoMessage>::new(SurfaceNode::scroll_area(
            31,
            SurfaceNode::text(
                100,
                "Long content",
                WidgetSizing::fixed(Vector2::new(320.0, 400.0)),
            ),
        ))),
        |surface| Arc::clone(surface),
        |_, _message| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 96.0));
    let content = runtime.layout().rects[&100];
    let viewport = runtime.layout().viewport_bounds[&31];
    assert!(content.width() > viewport.width());
    assert!(content.height() > viewport.height());
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
    assert_eq!(
        runtime
            .paint_plan(&Default::default())
            .primitives
            .iter()
            .filter(|primitive| {
                matches!(primitive, PaintPrimitive::FillRect(fill) if fill.widget_id == 31)
            })
            .count(),
        1,
        "legacy default overflow paints exactly one vertical bar"
    );

    runtime.dispatch_event(Event::PointerPress {
        position: thumb.center(),
        button: PointerButton::Primary,
        modifiers: Default::default(),
        timestamp: None,
    });
    assert_eq!(runtime.hovered_scroll_affordance(), Some(31));
    assert!(
        runtime.take_repaint_requested(),
        "pressing the painted scroll thumb should request a redraw"
    );
    runtime.dispatch_event(Event::pointer_move(Point::new(
        thumb.center().x,
        thumb.center().y + 36.0,
    )));
    assert!(
        runtime.take_repaint_requested(),
        "dragging the painted scroll thumb should request a redraw"
    );
    runtime.dispatch_event(Event::PointerRelease {
        position: Point::new(thumb.center().x, thumb.center().y + 36.0),
        button: PointerButton::Primary,
        modifiers: Default::default(),
        timestamp: None,
    });

    let after = runtime.layout().rects[&100];
    assert_eq!(after.min.x, before.min.x);
    assert!(after.min.y < before.min.y);
}

#[test]
fn scrollbar_release_settles_only_after_effective_movement() {
    let settled = Rc::new(Cell::new(0_usize));
    let mut runtime = SurfaceRuntime::new(
        declarative_runtime_bridge(
            Rc::clone(&settled),
            move |_| {
                crate::arc_surface(UiSurface::<DemoMessage>::new(
                    SurfaceNode::scroll_area(
                        31,
                        SurfaceNode::text(
                            32,
                            "Long content",
                            WidgetSizing::fixed(Vector2::new(180.0, 400.0)),
                        ),
                    )
                    .on_offset_settled(|_| DemoMessage::ScrollSettled),
                ))
            },
            |settled, message| {
                if message == DemoMessage::ScrollSettled {
                    settled.set(settled.get() + 1);
                }
            },
        ),
        Vector2::new(220.0, 96.0),
    );
    let thumb = runtime
        .paint_plan(&Default::default())
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) if fill.widget_id == 31 => Some(fill.rect),
            _ => None,
        })
        .expect("scroll area should paint a draggable thumb");

    runtime.dispatch_event(Event::primary_press(thumb.center()));
    runtime.dispatch_event(Event::primary_release(thumb.center()));
    assert_eq!(settled.get(), 0);

    runtime.dispatch_event(Event::primary_press(thumb.center()));
    runtime.dispatch_event(Event::pointer_move(Point::new(
        thumb.center().x,
        thumb.center().y + 36.0,
    )));
    runtime.dispatch_event(Event::primary_release(Point::new(
        thumb.center().x,
        thumb.center().y + 36.0,
    )));
    assert_eq!(settled.get(), 1);
}

#[test]
fn surface_runtime_highlights_painted_scrollbar_thumb_on_hover() {
    let bridge = declarative_runtime_bridge(
        crate::arc_surface(UiSurface::<DemoMessage>::new(SurfaceNode::scroll_area(
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

    runtime.dispatch_event(Event::pointer_move(thumb.center()));

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

    runtime.dispatch_event(Event::pointer_move(Point::new(8.0, 8.0)));
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
            crate::arc_surface(UiSurface::<DemoMessage>::new(node))
        },
        |state, message| match message {
            DemoMessage::Increment => *state = state.saturating_add(1),
            DemoMessage::ScrollSettled => {}
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

    runtime.dispatch_event(Event::pointer_move(thumb.center()));
    assert_eq!(runtime.hovered_scroll_affordance(), Some(31));

    runtime.dispatch_message(DemoMessage::Increment);

    assert_eq!(runtime.hovered_scroll_affordance(), None);
    assert!(
        !runtime.layout().rects.contains_key(&31),
        "the refreshed layout should no longer contain the hovered scroll area"
    );
}
