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

mod edit_lifecycle {
    use super::*;
    use radiant::runtime::ScrollEditBatch;
    use radiant::widgets::EditPhase;
    use std::cell::RefCell;
    type RecordedEdits = Rc<RefCell<Vec<ScrollEditBatch>>>;

    #[allow(clippy::arc_with_non_send_sync)]
    fn fixture(
        application: bool,
    ) -> (
        SurfaceRuntime<impl RuntimeBridge<ScrollEditBatch>, ScrollEditBatch>,
        RecordedEdits,
        Rc<Cell<f32>>,
    ) {
        let edits = Rc::new(RefCell::new(Vec::new()));
        let sink = Rc::clone(&edits);
        let height = Rc::new(Cell::new(400.0));
        let content_height = Rc::clone(&height);
        let bridge = declarative_runtime_bridge(
            (),
            move |_| {
                if application {
                    use radiant::prelude::{self as ui, IntoView};
                    return Arc::new(
                        ui::scroll(ui::text("content").size(180.0, content_height.get()).id(32))
                            .scroll_policy(
                                ScrollPolicy::default()
                                    .scrollbar_visibility(ScrollbarVisibility::Always),
                            )
                            .on_scroll_edit(|batch| batch)
                            .id(31)
                            .fill_width()
                            .fill_height()
                            .into_surface(),
                    );
                }

                Arc::new(UiSurface::new(
                    SurfaceNode::container(
                        31,
                        ContainerPolicy {
                            kind: ContainerKind::ScrollView,
                            overflow: OverflowPolicy::Scroll,
                            scroll_policy: ScrollPolicy::default()
                                .scrollbar_visibility(ScrollbarVisibility::Always),
                            ..ContainerPolicy::default()
                        },
                        vec![SurfaceChild::fill(SurfaceNode::text(
                            32,
                            "content",
                            WidgetSizing::fixed(Vector2::new(180.0, content_height.get())),
                        ))],
                    )
                    .on_scroll_edit(|batch| batch),
                ))
            },
            move |_, batch| sink.borrow_mut().push(batch),
        );
        (
            SurfaceRuntime::new(bridge, Vector2::new(220.0, 96.0)),
            edits,
            height,
        )
    }
    #[test]
    fn programmatic_offset_is_one_atomic_edit_and_clamped_noops_are_silent() {
        use radiant::widgets::{InteractionProvenance, InteractionSource};
        let (mut runtime, edits, _) = fixture(false);
        runtime.execute_command(Command::scroll_to(31, Vector2::new(0.0, 1000.0)));
        {
            let edits = edits.borrow();
            assert_eq!(edits.len(), 1);
            let batch = &edits[0];
            assert_eq!(
                phases(batch),
                [EditPhase::Begin, EditPhase::Update, EditPhase::Commit]
            );
            assert_eq!(
                batch.transaction().source(),
                InteractionSource::Programmatic
            );
            assert!(
                batch
                    .events()
                    .iter()
                    .all(|event| event.provenance == InteractionProvenance::Programmatic)
            );
            assert_eq!(batch.events()[0].value, Vector2::new(0.0, 0.0));
            assert_eq!(batch.events()[2].value, Vector2::new(0.0, 304.0));
            assert_eq!(batch.offset_update().unwrap().offset.y, 304.0);
        }
        runtime.execute_command(Command::scroll_to(31, Vector2::new(0.0, 2000.0)));
        runtime.execute_command(Command::scroll_to(31, Vector2::new(0.0, f32::NAN)));
        runtime.execute_command(Command::scroll_to(31, Vector2::new(0.0, f32::INFINITY)));
        runtime.execute_command(Command::scroll_to(999, Vector2::new(0.0, 20.0)));
        assert_eq!(edits.borrow().len(), 1);
        assert_eq!(runtime.layout().rects[&32].min.y, -304.0);
    }

    #[test]
    fn programmatic_replacement_cancels_pointer_owner_without_rolling_back() {
        let (mut runtime, edits, _) = fixture(false);
        let start = thumb(&runtime);
        runtime.dispatch_event(Event::primary_press(start));
        runtime.dispatch_event(Event::pointer_move(Point::new(start.x, start.y + 20.0)));
        runtime.execute_command(Command::scroll_to(31, Vector2::new(0.0, 80.0)));
        runtime.dispatch_event(Event::pointer_move(Point::new(start.x, start.y + 40.0)));
        runtime.dispatch_event(Event::primary_release(Point::new(start.x, start.y + 40.0)));
        let edits = edits.borrow();
        assert_eq!(edits.len(), 4);
        assert_eq!(phases(&edits[2]), [EditPhase::Cancel]);
        assert!(edits[2].offset_update().is_none());
        assert_eq!(edits[2].transaction(), edits[0].transaction());
        assert_eq!(
            phases(&edits[3]),
            [EditPhase::Begin, EditPhase::Update, EditPhase::Commit]
        );
        assert_ne!(edits[3].transaction(), edits[0].transaction());
        assert_eq!(runtime.layout().rects[&32].min.y, -80.0);
    }

    #[test]
    #[allow(clippy::arc_with_non_send_sync)]
    fn keyboard_repeats_are_distinct_atomic_edits_with_keyboard_provenance() {
        use radiant::widgets::{InteractionProvenance, KeyboardModifiers, WidgetKey};
        let edits = Rc::new(RefCell::new(Vec::<ScrollEditBatch>::new()));
        let sink = Rc::clone(&edits);
        let bridge = declarative_runtime_bridge(
            (),
            |_| {
                Arc::new(UiSurface::new(
                    SurfaceNode::scroll_area(
                        31,
                        SurfaceNode::button(
                            32,
                            "content",
                            WidgetSizing::fixed(Vector2::new(180.0, 400.0)),
                            None,
                        ),
                    )
                    .on_scroll_edit(Some),
                ))
            },
            move |_, batch: Option<ScrollEditBatch>| {
                if let Some(batch) = batch {
                    sink.borrow_mut().push(batch);
                }
            },
        );
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 96.0));
        assert!(runtime.focus_widget(32));
        runtime.execute_command(radiant::runtime::Command::scroll_to(
            31,
            Vector2::new(0.0, 0.0),
        ));
        edits.borrow_mut().clear();
        let timestamp = None;
        for repeat in [false, true] {
            runtime.dispatch_event(Event::KeyPress {
                key: WidgetKey::PageDown,
                modifiers: KeyboardModifiers::default(),
                repeat,
                timestamp,
            });
        }
        let edits = edits.borrow();
        assert_eq!(edits.len(), 2);
        assert_ne!(edits[0].transaction(), edits[1].transaction());
        for batch in edits.iter() {
            assert_eq!(
                phases(batch),
                [EditPhase::Begin, EditPhase::Update, EditPhase::Commit]
            );
            assert!(
                batch
                    .events()
                    .iter()
                    .all(|event| event.provenance == InteractionProvenance::Keyboard { timestamp })
            );
            assert_eq!(batch.offset_update().unwrap().metadata.timestamp, timestamp);
        }
    }

    #[test]
    #[allow(clippy::arc_with_non_send_sync)]
    fn replacement_during_pointer_cancellation_suppresses_stale_atomic_successor() {
        let edits = Rc::new(RefCell::new(Vec::<ScrollEditBatch>::new()));
        let sink = Rc::clone(&edits);
        let bridge = declarative_runtime_bridge(
            400.0,
            |height| {
                Arc::new(UiSurface::new(
                    SurfaceNode::container(
                        31,
                        ContainerPolicy {
                            kind: ContainerKind::ScrollView,
                            overflow: OverflowPolicy::Scroll,
                            scroll_policy: ScrollPolicy::default()
                                .scrollbar_visibility(ScrollbarVisibility::Always),
                            ..ContainerPolicy::default()
                        },
                        vec![SurfaceChild::fill(SurfaceNode::text(
                            32,
                            "content",
                            WidgetSizing::fixed(Vector2::new(180.0, *height)),
                        ))],
                    )
                    .on_scroll_edit(|batch| batch),
                ))
            },
            move |height, batch: ScrollEditBatch| {
                if batch
                    .events()
                    .last()
                    .is_some_and(|event| event.phase == EditPhase::Cancel)
                {
                    *height = 800.0;
                }
                sink.borrow_mut().push(batch);
            },
        );
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 96.0));
        let start = thumb(&runtime);
        runtime.dispatch_event(Event::primary_press(start));
        runtime.dispatch_event(Event::pointer_move(Point::new(start.x, start.y + 20.0)));
        runtime.execute_command(Command::scroll_to(31, Vector2::new(0.0, 80.0)));
        runtime.dispatch_event(Event::primary_release(Point::new(start.x, start.y + 40.0)));
        let edits = edits.borrow();
        assert_eq!(edits.len(), 3);
        assert_eq!(phases(&edits[2]), [EditPhase::Cancel]);
        assert!(edits[2].offset_update().is_none());
        assert_eq!(runtime.layout().rects[&32].height(), 800.0);
        assert_eq!(runtime.layout().rects[&32].min.y, -80.0);
    }

    fn wheel_sample(delta: f32, phase: Option<WheelPhase>) -> WheelSample {
        WheelSample::new(
            WheelDelta::Pixels(Vector2::new(0.0, delta)),
            phase,
            PointerModifiers::default(),
        )
        .unwrap()
    }

    #[test]
    fn wheel_sequence_keeps_its_container_outside_hit_bounds_and_ignores_stale_input() {
        let (mut runtime, edits, _) = fixture(false);
        assert!(runtime.wheel_or_scroll_at_with_sample(
            Point::new(20.0, 20.0),
            wheel_sample(8.0, Some(WheelPhase::Started))
        ));
        assert!(runtime.wheel_or_scroll_at_with_sample(
            Point::new(900.0, 900.0),
            wheel_sample(12.0, Some(WheelPhase::Changed))
        ));
        assert!(runtime.wheel_or_scroll_at_with_sample(
            Point::new(900.0, 900.0),
            wheel_sample(5.0, Some(WheelPhase::Ended))
        ));
        for phase in [
            WheelPhase::Ended,
            WheelPhase::Changed,
            WheelPhase::Cancelled,
        ] {
            assert!(!runtime.wheel_or_scroll_at_with_sample(
                Point::new(20.0, 20.0),
                wheel_sample(50.0, Some(phase))
            ));
        }
        let edits = edits.borrow();
        assert_eq!(edits.len(), 3);
        assert_eq!(phases(&edits[0]), [EditPhase::Begin, EditPhase::Update]);
        assert_eq!(phases(&edits[1]), [EditPhase::Update]);
        assert_eq!(phases(&edits[2]), [EditPhase::Update, EditPhase::Commit]);
        assert!(
            edits
                .iter()
                .all(|batch| batch.transaction() == edits[0].transaction())
        );
        assert_eq!(runtime.layout().rects[&32].min.y, -25.0);
    }

    #[test]
    fn wheel_cancellation_and_capture_loss_restore_the_starting_offset() {
        for capture_loss in [false, true] {
            let (mut runtime, edits, _) = fixture(false);
            runtime.execute_command(Command::scroll_to(31, Vector2::new(0.0, 40.0)));
            edits.borrow_mut().clear();
            runtime.wheel_or_scroll_at_with_sample(
                Point::new(20.0, 20.0),
                wheel_sample(20.0, Some(WheelPhase::Started)),
            );
            if capture_loss {
                runtime.dispatch_event(Event::pointer_capture_cancelled());
            } else {
                runtime.wheel_or_scroll_at_with_sample(
                    Point::new(900.0, 900.0),
                    wheel_sample(500.0, Some(WheelPhase::Cancelled)),
                );
            }
            let edits = edits.borrow();
            assert_eq!(edits.len(), 2);
            assert_eq!(phases(&edits[1]), [EditPhase::Cancel]);
            assert_eq!(edits[1].offset_update().unwrap().offset.y, 40.0);
            assert_eq!(runtime.layout().rects[&32].min.y, -40.0);
        }
    }

    #[test]
    fn replaced_wheel_geometry_cancels_without_rollback_or_rebinding() {
        let (mut runtime, edits, height) = fixture(false);
        runtime.wheel_or_scroll_at_with_sample(
            Point::new(20.0, 20.0),
            wheel_sample(20.0, Some(WheelPhase::Started)),
        );
        height.set(800.0);
        runtime.refresh();
        assert!(!runtime.wheel_or_scroll_at_with_sample(
            Point::new(20.0, 20.0),
            wheel_sample(30.0, Some(WheelPhase::Changed))
        ));
        assert_eq!(runtime.layout().rects[&32].min.y, -20.0);
        let edits = edits.borrow();
        assert_eq!(edits.len(), 2);
        assert_eq!(phases(&edits[1]), [EditPhase::Cancel]);
        assert!(edits[1].offset_update().is_none());
    }

    #[test]
    fn phase_less_and_discrete_wheel_samples_are_atomic() {
        let (mut runtime, edits, _) = fixture(false);
        for phase in [None, Some(WheelPhase::Discrete)] {
            runtime
                .wheel_or_scroll_at_with_sample(Point::new(20.0, 20.0), wheel_sample(12.0, phase));
        }
        let edits = edits.borrow();
        assert_eq!(edits.len(), 2);
        assert_ne!(edits[0].transaction(), edits[1].transaction());
        assert!(
            edits
                .iter()
                .all(|batch| phases(batch)
                    == [EditPhase::Begin, EditPhase::Update, EditPhase::Commit])
        );
        assert_eq!(runtime.layout().rects[&32].min.y, -24.0);
    }

    #[test]
    fn wheel_noop_boundaries_and_superseding_start_have_one_terminal_each() {
        let (mut runtime, edits, _) = fixture(false);
        let point = Point::new(20.0, 20.0);
        runtime.wheel_or_scroll_at_with_sample(point, wheel_sample(0.0, Some(WheelPhase::Started)));
        runtime.wheel_or_scroll_at_with_sample(point, wheel_sample(0.0, Some(WheelPhase::Ended)));
        {
            let edits = edits.borrow();
            assert_eq!(phases(&edits[0]), [EditPhase::Begin]);
            assert_eq!(phases(&edits[1]), [EditPhase::Commit]);
            assert!(edits.iter().all(|batch| batch.offset_update().is_none()));
        }
        edits.borrow_mut().clear();
        runtime
            .wheel_or_scroll_at_with_sample(point, wheel_sample(20.0, Some(WheelPhase::Started)));
        runtime.wheel_or_scroll_at_with_sample(point, wheel_sample(5.0, Some(WheelPhase::Started)));
        runtime.wheel_or_scroll_at_with_sample(point, wheel_sample(0.0, Some(WheelPhase::Ended)));
        let edits = edits.borrow();
        assert_eq!(edits.len(), 4);
        assert_eq!(phases(&edits[1]), [EditPhase::Cancel]);
        assert_eq!(edits[1].transaction(), edits[0].transaction());
        assert_ne!(edits[2].transaction(), edits[0].transaction());
        assert_eq!(edits[3].transaction(), edits[2].transaction());
        assert_eq!(runtime.layout().rects[&32].min.y, -5.0);
    }

    #[test]
    fn programmatic_wheel_replacement_and_competing_pointer_do_not_share_ownership() {
        let (mut runtime, edits, _) = fixture(false);
        let point = Point::new(20.0, 20.0);
        runtime
            .wheel_or_scroll_at_with_sample(point, wheel_sample(20.0, Some(WheelPhase::Started)));
        let bar = thumb(&runtime);
        runtime.dispatch_event(Event::primary_press(bar));
        runtime.dispatch_event(Event::pointer_move(Point::new(bar.x, bar.y + 20.0)));
        runtime.dispatch_event(Event::primary_release(Point::new(bar.x, bar.y + 20.0)));
        assert_eq!(edits.borrow().len(), 1);
        runtime.execute_command(Command::scroll_to(31, Vector2::new(0.0, 80.0)));
        assert!(
            !runtime.wheel_or_scroll_at_with_sample(
                point,
                wheel_sample(30.0, Some(WheelPhase::Changed))
            )
        );
        assert!(
            !runtime
                .wheel_or_scroll_at_with_sample(point, wheel_sample(30.0, Some(WheelPhase::Ended)))
        );
        let edits = edits.borrow();
        assert_eq!(edits.len(), 3);
        assert_eq!(phases(&edits[1]), [EditPhase::Cancel]);
        assert!(edits[1].offset_update().is_none());
        assert_eq!(
            phases(&edits[2]),
            [EditPhase::Begin, EditPhase::Update, EditPhase::Commit]
        );
        assert_eq!(runtime.layout().rects[&32].min.y, -80.0);
    }

    #[test]
    #[allow(clippy::arc_with_non_send_sync)]
    fn nested_wheel_chain_commits_or_rolls_back_each_owner_after_parent_translation() {
        for cancel in [false, true] {
            let edits = Rc::new(RefCell::new(Vec::<ScrollEditBatch>::new()));
            let sink = Rc::clone(&edits);
            let bridge = declarative_runtime_bridge(
                (),
                |_| Arc::new(UiSurface::new(nested_wheel_surface())),
                move |_, batch| sink.borrow_mut().push(batch),
            );
            let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 96.0));
            let before_inner = runtime.layout().rects[&42];
            let before_outer = runtime.layout().rects[&32];
            runtime.wheel_or_scroll_at_with_sample(
                Point::new(20.0, 20.0),
                wheel_sample(1000.0, Some(WheelPhase::Started)),
            );
            {
                let edits = edits.borrow();
                assert_eq!(edits.len(), 2);
                assert_eq!([edits[0].node_id(), edits[1].node_id()], [41, 31]);
                assert!(
                    edits
                        .iter()
                        .all(|batch| batch.offset_update().unwrap().offset.y > 0.0)
                );
            }
            runtime.wheel_or_scroll_at_with_sample(
                Point::new(900.0, 900.0),
                wheel_sample(
                    0.0,
                    Some(if cancel {
                        WheelPhase::Cancelled
                    } else {
                        WheelPhase::Ended
                    }),
                ),
            );
            let edits = edits.borrow();
            assert_eq!(edits.len(), 4);
            for index in 0..2 {
                assert_eq!(edits[index + 2].transaction(), edits[index].transaction());
                assert_eq!(
                    phases(&edits[index + 2]),
                    [if cancel {
                        EditPhase::Cancel
                    } else {
                        EditPhase::Commit
                    }]
                );
            }
            if cancel {
                assert_eq!(runtime.layout().rects[&42], before_inner);
                assert_eq!(runtime.layout().rects[&32], before_outer);
            }
        }
    }

    #[test]
    #[allow(clippy::arc_with_non_send_sync)]
    fn controlled_wheel_echo_preserves_owner_and_new_value_retires_it() {
        use radiant::layout::Controlled;
        let controlled = Rc::new(Cell::new((0.0_f32, 1_u64)));
        let projected = Rc::clone(&controlled);
        let reduced = Rc::clone(&controlled);
        let edits = Rc::new(RefCell::new(Vec::<ScrollEditBatch>::new()));
        let sink = Rc::clone(&edits);
        let bridge = declarative_runtime_bridge(
            (),
            move |_| {
                let (offset, generation) = projected.get();
                Arc::new(UiSurface::new(
                    SurfaceNode::scroll_area(
                        31,
                        SurfaceNode::text(
                            32,
                            "content",
                            WidgetSizing::fixed(Vector2::new(180.0, 400.0)),
                        ),
                    )
                    .controlled_offset(Controlled::new(Vector2::new(0.0, offset), generation))
                    .on_scroll_edit(|batch| batch),
                ))
            },
            move |_, batch: ScrollEditBatch| {
                if let Some(update) = batch.offset_update() {
                    reduced.set((update.offset.y, reduced.get().1 + 1));
                }
                sink.borrow_mut().push(batch);
            },
        );
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 96.0));
        let point = Point::new(20.0, 20.0);
        runtime
            .wheel_or_scroll_at_with_sample(point, wheel_sample(20.0, Some(WheelPhase::Started)));
        runtime
            .wheel_or_scroll_at_with_sample(point, wheel_sample(10.0, Some(WheelPhase::Changed)));
        assert_eq!(controlled.get().0, 30.0);
        controlled.set((80.0, 10));
        runtime.refresh();
        assert!(
            !runtime.wheel_or_scroll_at_with_sample(
                point,
                wheel_sample(20.0, Some(WheelPhase::Changed))
            )
        );
        let edits = edits.borrow();
        assert_eq!(edits.len(), 3);
        assert!(
            edits
                .iter()
                .all(|batch| batch.transaction() == edits[0].transaction())
        );
        assert_eq!(phases(&edits[2]), [EditPhase::Cancel]);
        assert!(edits[2].offset_update().is_none());
        assert_eq!(controlled.get().0, 80.0);
        assert_eq!(runtime.layout().rects[&32].min.y, -80.0);
    }

    fn nested_wheel_surface() -> SurfaceNode<ScrollEditBatch> {
        use radiant::layout::{SizeModeMain, SlotParams};
        let inner = SurfaceNode::scroll_area(
            41,
            SurfaceNode::text(42, "inner", WidgetSizing::fixed(Vector2::new(180.0, 400.0))),
        )
        .on_scroll_edit(|batch| batch);
        let content = SurfaceNode::column(
            32,
            0.0,
            vec![
                SurfaceChild::new(
                    SlotParams {
                        size_main: SizeModeMain::Fixed(80.0),
                        ..intrinsic_slot()
                    },
                    inner,
                ),
                SurfaceChild::new(
                    intrinsic_slot(),
                    SurfaceNode::text(43, "outer", WidgetSizing::fixed(Vector2::new(180.0, 400.0))),
                ),
            ],
        );
        SurfaceNode::scroll_area(31, content).on_scroll_edit(|batch| batch)
    }

    #[test]
    #[allow(clippy::arc_with_non_send_sync)]
    fn callback_replacement_never_cancels_an_ancestor_before_its_begin_is_delivered() {
        use radiant::layout::Controlled;
        let generation = Rc::new(Cell::new(1_u64));
        let projected = Rc::clone(&generation);
        let reduced = Rc::clone(&generation);
        let edits = Rc::new(RefCell::new(Vec::<ScrollEditBatch>::new()));
        let sink = Rc::clone(&edits);
        let bridge = declarative_runtime_bridge(
            (),
            move |_| {
                Arc::new(UiSurface::new(nested_wheel_surface().controlled_offset(
                    Controlled::new(Vector2::new(0.0, 0.0), projected.get()),
                )))
            },
            move |_, batch: ScrollEditBatch| {
                if batch.node_id() == 41 && reduced.get() == 1 {
                    reduced.set(2);
                }
                sink.borrow_mut().push(batch);
            },
        );
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 96.0));
        runtime.wheel_or_scroll_at_with_sample(
            Point::new(20.0, 20.0),
            wheel_sample(1000.0, Some(WheelPhase::Started)),
        );
        let edits = edits.borrow();
        assert_eq!(edits.len(), 2);
        assert_eq!([edits[0].node_id(), edits[1].node_id()], [41, 41]);
        assert_eq!(phases(&edits[0]), [EditPhase::Begin, EditPhase::Update]);
        assert_eq!(phases(&edits[1]), [EditPhase::Cancel]);
        assert_eq!(edits[0].transaction(), edits[1].transaction());
        assert!(edits[1].offset_update().is_none());
        assert_eq!(runtime.layout().rects[&32].min.y, 0.0);
    }

    fn thumb<Bridge: RuntimeBridge<ScrollEditBatch>>(
        runtime: &SurfaceRuntime<Bridge, ScrollEditBatch>,
    ) -> Point {
        runtime
            .paint_plan(&Default::default())
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::FillRect(fill) if fill.widget_id == 31 => Some(fill.rect.center()),
                _ => None,
            })
            .expect("visible scrollbar thumb")
    }
    fn phases(batch: &ScrollEditBatch) -> Vec<EditPhase> {
        batch.events().iter().map(|event| event.phase).collect()
    }

    #[test]
    fn container_scrollbar_release_batches_final_motion_and_commits_once() {
        let (mut runtime, edits, _) = fixture(false);
        let point = thumb(&runtime);
        runtime.dispatch_event(Event::primary_press(point));
        runtime.dispatch_event(Event::primary_press(point));
        runtime.dispatch_event(Event::primary_release(Point::new(point.x, point.y + 25.0)));
        runtime.dispatch_event(Event::primary_release(Point::new(point.x, point.y + 40.0)));
        let edits = edits.borrow();
        assert_eq!(edits.len(), 2);
        assert_eq!(phases(&edits[0]), [EditPhase::Begin]);
        assert_eq!(phases(&edits[1]), [EditPhase::Update, EditPhase::Commit]);
        assert_eq!(edits[0].transaction(), edits[1].transaction());
        assert!(edits[1].offset_update().unwrap().offset.y > 0.0);
    }

    #[test]
    fn container_scrollbar_capture_loss_rolls_back_without_a_late_commit() {
        let (mut runtime, edits, _) = fixture(false);
        let point = thumb(&runtime);
        let original = runtime.layout().rects[&32];
        runtime.dispatch_event(Event::primary_press(point));
        runtime.dispatch_event(Event::pointer_move(Point::new(point.x, point.y + 25.0)));
        assert_ne!(runtime.layout().rects[&32], original);
        runtime.dispatch_event(Event::pointer_capture_cancelled());
        assert_eq!(runtime.layout().rects[&32], original);
        runtime.dispatch_event(Event::primary_release(Point::new(point.x, point.y + 40.0)));
        let edits = edits.borrow();
        assert_eq!(edits.len(), 3);
        assert_eq!(phases(&edits[2]), [EditPhase::Cancel]);
        assert_eq!(edits[2].offset_update().unwrap().offset, Vector2::default());
        assert!(edits[2].offset_update().unwrap().delta.y < 0.0);
        assert!(
            edits
                .iter()
                .all(|edit| edit.transaction() == edits[0].transaction())
        );
    }

    #[test]
    fn container_scrollbar_noop_cancel_remains_typed_without_offset_projection() {
        let (mut runtime, edits, _) = fixture(false);
        let point = thumb(&runtime);
        runtime.dispatch_event(Event::primary_press(point));
        runtime.dispatch_event(Event::pointer_capture_cancelled());
        runtime.dispatch_event(Event::pointer_capture_cancelled());
        let edits = edits.borrow();
        assert_eq!(edits.len(), 2);
        assert_eq!(phases(&edits[1]), [EditPhase::Cancel]);
        assert!(edits[1].offset_update().is_none());
    }

    #[test]
    fn container_scrollbar_geometry_replacement_retires_old_edit_without_rollback() {
        let (mut runtime, edits, height) = fixture(false);
        let point = thumb(&runtime);
        runtime.dispatch_event(Event::primary_press(point));
        runtime.dispatch_event(Event::pointer_move(Point::new(point.x, point.y + 20.0)));
        height.set(800.0);
        runtime.refresh();
        let projected = runtime.layout().rects[&32];
        runtime.dispatch_event(Event::pointer_move(Point::new(point.x, point.y + 30.0)));
        assert_eq!(runtime.layout().rects[&32], projected);
        runtime.dispatch_event(Event::primary_release(Point::new(point.x, point.y + 40.0)));
        let edits = edits.borrow();
        assert_eq!(edits.len(), 3);
        assert_eq!(phases(&edits[2]), [EditPhase::Cancel]);
        assert!(edits[2].offset_update().is_none());
    }

    #[test]
    fn application_scroll_edit_modifier_lowers_to_runtime_pointer_lifecycle() {
        let (mut runtime, edits, _) = fixture(true);
        let point = thumb(&runtime);
        runtime.dispatch_event(Event::primary_press(point));
        runtime.dispatch_event(Event::primary_release(Point::new(point.x, point.y + 20.0)));
        let edits = edits.borrow();
        assert_eq!(edits.len(), 2);
        assert_eq!(phases(&edits[0]), [EditPhase::Begin]);
        assert_eq!(phases(&edits[1]), [EditPhase::Update, EditPhase::Commit]);
    }
}
