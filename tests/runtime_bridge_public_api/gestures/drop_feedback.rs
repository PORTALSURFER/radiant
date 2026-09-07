use super::*;
use radiant::{
    application::{DragSource, DropTarget, button, row},
    runtime::{
        DragOperation, DropDecision, DropInsertion, DropInsertionAxis, DropInsertionSide,
        DropPhase, DropTargetFeedback, PaintPrimitive,
    },
    theme::ThemeTokens,
};

fn bridge(
    decision: DropDecision,
    feedback: Option<DropTargetFeedback>,
    emit: bool,
    shown: Rc<Cell<bool>>,
) -> impl radiant::runtime::RuntimeBridge<()> {
    bridge_with_removal(decision, feedback, emit, shown, false, None)
}
fn bridge_with_axis(
    decision: DropDecision,
    feedback: Option<DropTargetFeedback>,
    emit: bool,
    shown: Rc<Cell<bool>>,
    axis: DropInsertionAxis,
) -> impl radiant::runtime::RuntimeBridge<()> {
    bridge_with_removal(decision, feedback, emit, shown, false, Some(axis))
}
fn bridge_with_removal(
    decision: DropDecision,
    feedback: Option<DropTargetFeedback>,
    emit: bool,
    shown: Rc<Cell<bool>>,
    remove_on_event: bool,
    insertion_axis: Option<DropInsertionAxis>,
) -> impl radiant::runtime::RuntimeBridge<()> {
    let remove = shown.clone();
    radiant::app(())
        .view(move |_| {
            let source = button("Source")
                .filter_mapped(|_| None::<()>)
                .width(100.0)
                .height(40.0)
                .id(1)
                .drag_source(DragSource::new(String::from("payload")))
                .id(10);
            let target = button("Target")
                .filter_mapped(|_| None::<()>)
                .width(100.0)
                .height(40.0)
                .id(2);
            let target = if shown.get() {
                let mut drop = DropTarget::<String, ()>::new()
                    .negotiate_with_revision(decision, move |_, _| decision)
                    .on_event_with_revision(emit, move |_| emit.then_some(()));
                if let Some(feedback) = feedback {
                    drop = drop.feedback(feedback);
                }
                if let Some(axis) = insertion_axis {
                    drop = drop.insertion_axis(axis);
                }
                target.drop_target(drop).id(20)
            } else {
                target
            };
            row([source, target]).spacing(0.0).id(30)
        })
        .update(move |_, ()| {
            if remove_on_event {
                remove.set(false);
            }
        })
        .into_bridge()
}
fn start<B: radiant::runtime::RuntimeBridge<()>>(
    runtime: &mut SurfaceRuntime<B, ()>,
) -> radiant::runtime::GestureSequenceToken {
    let token = runtime
        .dispatch_gesture_request(GestureRequest::new(sample(
            GestureKind::Pan,
            GesturePhase::Started,
            Vector2::default(),
        )))
        .token()
        .unwrap();
    send(runtime, token, GesturePhase::Changed, 110.0);
    token
}
fn send<B: radiant::runtime::RuntimeBridge<()>>(
    runtime: &mut SurfaceRuntime<B, ()>,
    token: radiant::runtime::GestureSequenceToken,
    phase: GesturePhase,
    x: f32,
) {
    runtime.dispatch_gesture_request(
        GestureRequest::new(sample(GestureKind::Pan, phase, Vector2::new(x, 0.0)))
            .with_token(token),
    );
}
fn overlay<B: radiant::runtime::RuntimeBridge<()>>(
    runtime: &SurfaceRuntime<B, ()>,
    theme: &ThemeTokens,
) -> Vec<PaintPrimitive> {
    let mut paint = Vec::new();
    runtime.runtime_overlay_paint_into(theme, &mut paint);
    paint
}
fn feedback_color(paint: &[PaintPrimitive]) -> Option<radiant::gui::types::Rgba8> {
    paint.iter().find_map(|p| match p {
        PaintPrimitive::StrokeRect(stroke) if stroke.widget_id == 20 => Some(stroke.color),
        PaintPrimitive::FillRect(fill) if fill.widget_id == 20 => Some(fill.color),
        _ => None,
    })
}

fn insertion_bridge(
    axis: DropInsertionAxis,
    feedback: Option<DropTargetFeedback>,
    insertions: Rc<RefCell<Vec<(DropPhase, Option<DropInsertion>)>>>,
    callbacks: Rc<Cell<u32>>,
) -> impl radiant::runtime::RuntimeBridge<()> {
    radiant::app(())
        .view(move |_| {
            let source = button("Source")
                .filter_mapped(|_| None::<()>)
                .width(100.0)
                .height(40.0)
                .id(1)
                .drag_source(DragSource::new(String::from("payload")))
                .id(10);
            let mut target = DropTarget::<String, ()>::new()
                .insertion_axis(axis)
                .negotiate_with_revision((), {
                    let callbacks = callbacks.clone();
                    move |_, context| {
                        callbacks.set(callbacks.get() + 1);
                        assert!(context.insertion().is_some());
                        DropDecision::Accepted(DragOperation::Copy)
                    }
                })
                .on_event_with_revision((), {
                    let insertions = insertions.clone();
                    let callbacks = callbacks.clone();
                    move |event| {
                        callbacks.set(callbacks.get() + 1);
                        insertions
                            .borrow_mut()
                            .push((event.phase(), event.context().insertion()));
                        None
                    }
                });
            if let Some(feedback) = feedback {
                target = target.feedback(feedback);
            }
            row([
                source,
                button("Target")
                    .filter_mapped(|_| None::<()>)
                    .width(100.0)
                    .height(40.0)
                    .id(2)
                    .drop_target(target)
                    .id(20),
            ])
            .spacing(0.0)
            .id(30)
        })
        .update(|_, ()| {})
        .into_bridge()
}

fn geometry_reprojection_insertion_bridge(
    narrow: Rc<Cell<bool>>,
) -> impl radiant::runtime::RuntimeBridge<()> {
    let update_narrow = narrow.clone();
    radiant::app(())
        .view(move |_| {
            let source = button("Source")
                .filter_mapped(|_| None::<()>)
                .width(100.0)
                .height(40.0)
                .id(1)
                .drag_source(DragSource::new(String::from("payload")))
                .id(10);
            let width = narrow.get().then_some(20.0).unwrap_or(100.0);
            let target = button("Target")
                .filter_mapped(|_| None::<()>)
                .width(width)
                .height(40.0)
                .id(2)
                .drop_target(
                    DropTarget::<String, ()>::new()
                        .feedback(DropTargetFeedback::themed())
                        .insertion_axis(DropInsertionAxis::Horizontal)
                        .negotiate_with_revision((), |_, _| {
                            DropDecision::Accepted(DragOperation::Copy)
                        })
                        .on_event_with_revision((), |_| Some(())),
                )
                .id(20);
            row([source, target]).spacing(0.0).id(30)
        })
        .update(move |_, ()| update_narrow.set(true))
        .into_bridge()
}

#[test]
fn insertion_context_uses_both_axes_midpoint_after_and_final_drop_input() {
    for (axis, after) in [
        (DropInsertionAxis::Horizontal, 150.0),
        (DropInsertionAxis::Vertical, 20.0),
    ] {
        let insertions = Rc::new(RefCell::new(Vec::new()));
        let callbacks = Rc::new(Cell::new(0));
        let mut runtime = SurfaceRuntime::new(
            insertion_bridge(axis, None, insertions.clone(), callbacks),
            Vector2::new(240.0, 80.0),
        );
        let token = start(&mut runtime);
        if axis == DropInsertionAxis::Vertical {
            // The initial target entry at y=0 is Before; a later y=20 is the midpoint.
            runtime.dispatch_gesture_request(
                GestureRequest::new(sample(
                    GestureKind::Pan,
                    GesturePhase::Changed,
                    Vector2::new(110.0, after),
                ))
                .with_token(token),
            );
        } else {
            send(&mut runtime, token, GesturePhase::Changed, after);
        }
        if axis == DropInsertionAxis::Vertical {
            runtime.dispatch_gesture_request(
                GestureRequest::new(sample(
                    GestureKind::Pan,
                    GesturePhase::Ended,
                    Vector2::new(110.0, after),
                ))
                .with_token(token),
            );
        } else {
            send(&mut runtime, token, GesturePhase::Ended, after);
        }
        let entries = insertions.borrow();
        assert!(entries.iter().any(|(_, insertion)| {
            insertion.is_some_and(|insertion| {
                insertion.axis() == axis && insertion.side() == DropInsertionSide::Before
            })
        }));
        assert!(entries.iter().any(|(_, insertion)| {
            insertion.is_some_and(|insertion| {
                insertion.axis() == axis && insertion.side() == DropInsertionSide::After
            })
        }));
        assert!(
            matches!(entries.last(), Some((DropPhase::Dropped, Some(insertion))) if insertion.axis() == axis && insertion.side() == DropInsertionSide::After)
        );
    }
}

#[test]
fn insertion_context_does_not_require_feedback_and_feedback_paints_an_edge_marker() {
    let insertions = Rc::new(RefCell::new(Vec::new()));
    let callbacks = Rc::new(Cell::new(0));
    let mut runtime = SurfaceRuntime::new(
        insertion_bridge(
            DropInsertionAxis::Horizontal,
            Some(DropTargetFeedback::themed()),
            insertions.clone(),
            callbacks.clone(),
        ),
        Vector2::new(240.0, 80.0),
    );
    let token = start(&mut runtime);
    let paint = overlay(&runtime, &ThemeTokens::default());
    let marker = paint.iter().find_map(|primitive| match primitive {
        PaintPrimitive::FillRect(fill) if fill.widget_id == 20 => Some(fill.rect),
        _ => None,
    });
    assert_eq!(marker.map(|rect| rect.width()), Some(2.0));
    let callback_count = callbacks.get();
    let _ = overlay(&runtime, &ThemeTokens::default());
    assert_eq!(callbacks.get(), callback_count);
    send(&mut runtime, token, GesturePhase::Ended, 0.0);
    assert!(!insertions.borrow().is_empty());
}

#[test]
fn compatible_target_callback_requalifies_insertion_against_new_geometry() {
    let narrow = Rc::new(Cell::new(false));
    let mut runtime = SurfaceRuntime::new(
        geometry_reprojection_insertion_bridge(narrow.clone()),
        Vector2::new(240.0, 80.0),
    );
    let _token = start(&mut runtime);
    assert!(narrow.get());
    let marker = overlay(&runtime, &ThemeTokens::default())
        .into_iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) if fill.widget_id == 20 => Some(fill.rect),
            _ => None,
        })
        .expect("requalified insertion marker");
    assert_eq!(marker.min.x, 118.0);
    assert_eq!(marker.max.x, 120.0);
}

#[test]
fn drop_feedback_resolves_each_decision_and_survives_compatible_event_projection() {
    let feedback = DropTargetFeedback::themed();
    for emit in [false, true] {
        for (decision, style) in [
            (
                DropDecision::Accepted(DragOperation::Copy),
                feedback.accepted,
            ),
            (DropDecision::Pending, feedback.pending),
            (DropDecision::Rejected, feedback.rejected),
        ] {
            let mut runtime = SurfaceRuntime::new(
                bridge(decision, Some(feedback), emit, Rc::new(Cell::new(true))),
                Vector2::new(240.0, 80.0),
            );
            let token = start(&mut runtime);
            let theme = ThemeTokens::default();
            let expected = radiant::widgets::resolve_widget_visual_tokens(
                &theme,
                style,
                radiant::widgets::WidgetState {
                    active: true,
                    selected: true,
                    ..Default::default()
                },
            )
            .emphasis;
            assert_eq!(feedback_color(&overlay(&runtime, &theme)), Some(expected));
            send(&mut runtime, token, GesturePhase::Ended, 0.0);
            assert_eq!(feedback_color(&overlay(&runtime, &theme)), None);
        }
    }
}

#[test]
fn drop_feedback_is_opt_in_and_pointer_motion_needs_no_projection() {
    for feedback in [None, Some(DropTargetFeedback::themed())] {
        let mut runtime = SurfaceRuntime::new(
            bridge(
                DropDecision::Pending,
                feedback,
                false,
                Rc::new(Cell::new(true)),
            ),
            Vector2::new(240.0, 80.0),
        );
        let token = start(&mut runtime);
        let counters = runtime.refresh_counters();
        send(&mut runtime, token, GesturePhase::Changed, 2.0);
        assert_eq!(runtime.refresh_counters(), counters);
        assert_eq!(
            feedback_color(&overlay(&runtime, &ThemeTokens::default())).is_some(),
            feedback.is_some()
        );
        send(&mut runtime, token, GesturePhase::Changed, -100.0);
        assert!(feedback_color(&overlay(&runtime, &ThemeTokens::default())).is_none());
    }
}

#[test]
fn drop_feedback_clips_to_viewport_and_hides_retired_target() {
    let shown = Rc::new(Cell::new(true));
    let mut runtime = SurfaceRuntime::new(
        bridge_with_axis(
            DropDecision::Pending,
            Some(DropTargetFeedback::themed()),
            false,
            shown.clone(),
            DropInsertionAxis::Horizontal,
        ),
        Vector2::new(150.0, 80.0),
    );
    let token = start(&mut runtime);
    let paint = overlay(&runtime, &ThemeTokens::default());
    assert!(feedback_color(&paint).is_some());
    assert!(paint.iter().any(|p| matches!(p, PaintPrimitive::ClipStart(clip) if clip.node_id == 20 && clip.rect.max.x == 150.0)));
    shown.set(false);
    runtime.refresh();
    assert!(feedback_color(&overlay(&runtime, &ThemeTokens::default())).is_none());
    send(&mut runtime, token, GesturePhase::Changed, 0.0);
    assert!(feedback_color(&overlay(&runtime, &ThemeTokens::default())).is_none());
}

#[test]
fn drop_feedback_resize_requires_fresh_input_and_focus_loss_clears_it() {
    let mut runtime = SurfaceRuntime::new(
        bridge_with_axis(
            DropDecision::Pending,
            Some(DropTargetFeedback::themed()),
            false,
            Rc::new(Cell::new(true)),
            DropInsertionAxis::Horizontal,
        ),
        Vector2::new(240.0, 80.0),
    );
    let token = start(&mut runtime);
    let theme = ThemeTokens::default();
    assert!(feedback_color(&overlay(&runtime, &theme)).is_some());
    runtime.set_viewport(Vector2::new(230.0, 80.0));
    assert!(feedback_color(&overlay(&runtime, &theme)).is_none());
    send(&mut runtime, token, GesturePhase::Changed, 0.0);
    assert!(feedback_color(&overlay(&runtime, &theme)).is_some());
    runtime.clear_focus();
    assert!(feedback_color(&overlay(&runtime, &theme)).is_none());
}

#[test]
fn drop_feedback_target_removal_during_enter_never_paints_retired_outline() {
    let mut runtime = SurfaceRuntime::new(
        bridge_with_removal(
            DropDecision::Pending,
            Some(DropTargetFeedback::themed()),
            true,
            Rc::new(Cell::new(true)),
            true,
            Some(DropInsertionAxis::Horizontal),
        ),
        Vector2::new(240.0, 80.0),
    );
    let token = start(&mut runtime);
    assert!(feedback_color(&overlay(&runtime, &ThemeTokens::default())).is_none());
    send(&mut runtime, token, GesturePhase::Changed, 0.0);
    assert!(feedback_color(&overlay(&runtime, &ThemeTokens::default())).is_none());
}
