use crate::gpu_content::demo_gpu_content;
use crate::model::{DemoMessage, DemoState};
use crate::selection_overlay::SelectionOverlay;
use crate::transient_overlay::{paint_transient_blob, triangle_wave};
use crate::view::{SURFACE_HEIGHT, SURFACE_WIDTH};
use radiant::prelude::*;
use radiant::runtime::{RuntimeBridge, SurfaceRuntime, TransientOverlayContext};
use radiant::theme::ThemeTokens;
use std::{cell::Cell, rc::Rc, time::Duration};

#[test]
fn triangle_wave_bounces_between_edges() {
    assert_eq!(triangle_wave(0.0), 0.0);
    assert_eq!(triangle_wave(0.25), 0.5);
    assert_eq!(triangle_wave(0.5), 1.0);
    assert_eq!(triangle_wave(0.75), 0.5);
}

#[test]
fn animated_transient_overlay_requests_paint_only_frames() {
    let painted = Rc::new(Cell::new(false));
    let painted_probe = Rc::clone(&painted);
    let mut runtime = SurfaceRuntime::new(
        radiant::app(DemoState::default())
            .view(|state| {
                stack([
                    gpu_surface(42, 1, demo_gpu_content())
                        .id(10)
                        .size(SURFACE_WIDTH, SURFACE_HEIGHT),
                    custom_widget_mapped(SelectionOverlay::new(state), |message: DemoMessage| {
                        message
                    })
                    .id(11)
                    .size(SURFACE_WIDTH, SURFACE_HEIGHT),
                ])
                .id(12)
                .size(SURFACE_WIDTH, SURFACE_HEIGHT)
            })
            .animated_transient_overlay_at(
                60,
                |state| state.running,
                move |state, context, primitives| {
                    assert!(state.running);
                    paint_transient_blob(state, context.plan, context.animation_time, primitives);
                    painted_probe.set(true);
                },
            )
            .update_command(|_state: &mut DemoState, _message| Command::none())
            .into_bridge(),
        Vector2::new(SURFACE_WIDTH, SURFACE_HEIGHT),
    );

    let activity = runtime.bridge_mut().animation_activity();
    assert!(activity.needs_animation());
    assert!(!activity.needs_frame_message());
    assert_eq!(activity.target_fps(), Some(60));
    assert!(
        !runtime.bridge_mut().queue_animation_frame(),
        "paint-only overlay animation must not enqueue app frame messages"
    );

    let plan = runtime.paint_plan(&ThemeTokens::default());
    let mut primitives = Vec::new();
    runtime.bridge_mut().paint_transient_overlay(
        TransientOverlayContext::new(
            &plan,
            Vector2::new(SURFACE_WIDTH, SURFACE_HEIGHT),
            Duration::from_millis(250),
        ),
        &mut primitives,
    );

    assert!(painted.get());
    assert!(
        primitives.iter().any(
            |primitive| matches!(primitive, PaintPrimitive::FillRect(rect) if rect.widget_id == 11)
        ),
        "transient overlay painter should append blob primitives against the cached paint plan"
    );
}
