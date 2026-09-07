use super::*;
use radiant::{
    application::{DragSource, DropTarget, button, row},
    runtime::{DragOperation, DropDecision, DropTargetFeedback, PaintPrimitive},
    theme::ThemeTokens,
};

fn bridge(
    decision: DropDecision,
    feedback: Option<DropTargetFeedback>,
    emit: bool,
    shown: Rc<Cell<bool>>,
) -> impl radiant::runtime::RuntimeBridge<()> {
    bridge_with_removal(decision, feedback, emit, shown, false)
}
fn bridge_with_removal(
    decision: DropDecision,
    feedback: Option<DropTargetFeedback>,
    emit: bool,
    shown: Rc<Cell<bool>>,
    remove_on_event: bool,
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
        _ => None,
    })
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
        bridge(
            DropDecision::Pending,
            Some(DropTargetFeedback::themed()),
            false,
            shown.clone(),
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
        bridge(
            DropDecision::Pending,
            Some(DropTargetFeedback::themed()),
            false,
            Rc::new(Cell::new(true)),
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
        ),
        Vector2::new(240.0, 80.0),
    );
    let token = start(&mut runtime);
    assert!(feedback_color(&overlay(&runtime, &ThemeTokens::default())).is_none());
    send(&mut runtime, token, GesturePhase::Changed, 0.0);
    assert!(feedback_color(&overlay(&runtime, &ThemeTokens::default())).is_none());
}
