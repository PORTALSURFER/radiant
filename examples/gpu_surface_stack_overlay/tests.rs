use super::*;
use radiant::runtime::{Event, RuntimeBridge, SurfaceRuntime, TransientOverlayContext};
use radiant::theme::ThemeTokens;
use radiant::widgets::TextWidget;
use std::{cell::Cell, rc::Rc};

#[test]
fn resize_selection_keeps_minimum_width() {
    let mut overlay = SelectionOverlay::new(&DemoState {
        selection_start: 0.22,
        selection_end: 0.68,
        ..DemoState::default()
    });
    overlay.drag_handle = Some(ResizeHandle::Start);

    overlay.resize_selection(0.67);

    assert!(overlay.selection_end - overlay.selection_start >= 0.04);
}

#[test]
fn resize_preview_stays_widget_local_until_release() {
    let mut overlay = SelectionOverlay::new(&DemoState::default());
    overlay.drag_handle = Some(ResizeHandle::Start);
    let bounds = Rect::from_min_size(
        Point::new(0.0, 0.0),
        Vector2::new(SURFACE_WIDTH, SURFACE_HEIGHT),
    );

    let output = overlay.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(SURFACE_WIDTH * 0.32, 10.0),
        },
    );

    assert!(output.is_none());
    assert_eq!(overlay.selection_start, 0.32);
    assert_eq!(overlay.drag_handle, Some(ResizeHandle::Start));
}

#[test]
fn resize_release_commits_final_selection_once() {
    let mut overlay = SelectionOverlay::new(&DemoState::default());
    overlay.drag_handle = Some(ResizeHandle::End);
    overlay.selection_end = 0.74;
    let bounds = Rect::from_min_size(
        Point::new(0.0, 0.0),
        Vector2::new(SURFACE_WIDTH, SURFACE_HEIGHT),
    );

    let output = overlay
        .handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(SURFACE_WIDTH * 0.74, 10.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        )
        .expect("release should emit a commit message");

    assert_eq!(overlay.drag_handle, None);
    assert_eq!(
        output.custom_ref::<DemoMessage>(),
        Some(&DemoMessage::CommitResize {
            start: 0.22,
            end: 0.74
        })
    );
}

#[test]
fn runtime_resize_drag_previews_locally_and_commits_once() {
    let mut runtime = SurfaceRuntime::new(
        radiant::app(DemoState::default())
            .view(|state| {
                column([
                    text(format!(
                        "selection {:.0}% - {:.0}%",
                        state.selection_start * 100.0,
                        state.selection_end * 100.0
                    ))
                    .id(2)
                    .height(32.0),
                    custom_widget_mapped(SelectionOverlay::new(state), |message: DemoMessage| {
                        message
                    })
                    .id(11)
                    .size(SURFACE_WIDTH, SURFACE_HEIGHT),
                ])
            })
            .update_command(|state: &mut DemoState, message| match message {
                DemoMessage::CommitResize { start, end } => {
                    state.commit_selection(start, end);
                    Command::request_repaint()
                }
                _ => Command::none(),
            })
            .into_bridge(),
        Vector2::new(SURFACE_WIDTH, SURFACE_HEIGHT + 32.0),
    );

    let handle_position = Point::new(SURFACE_WIDTH * 0.22, 44.0);
    let preview_position = Point::new(SURFACE_WIDTH * 0.32, 44.0);

    runtime.dispatch_event(Event::PointerPress {
        position: handle_position,
        button: PointerButton::Primary,
        modifiers: Default::default(),
    });
    runtime.dispatch_event(Event::PointerMove {
        position: preview_position,
    });

    let overlay = selection_overlay(&runtime);
    assert_eq!(overlay.selection_start, 0.32);
    assert_eq!(overlay.drag_handle, Some(ResizeHandle::Start));
    assert_eq!(
        text_widget(&runtime, 2).text,
        "selection 22% - 68%",
        "host state should not refresh on every drag pixel"
    );

    runtime.dispatch_event(Event::PointerRelease {
        position: preview_position,
        button: PointerButton::Primary,
        modifiers: Default::default(),
    });

    assert_eq!(selection_overlay(&runtime).drag_handle, None);
    assert_eq!(text_widget(&runtime, 2).text, "selection 32% - 68%");
}

#[test]
fn triangle_wave_bounces_between_edges() {
    assert_eq!(triangle_wave(0.0), 0.0);
    assert_eq!(triangle_wave(0.25), 0.5);
    assert_eq!(triangle_wave(0.5), 1.0);
    assert_eq!(triangle_wave(0.75), 0.5);
}

#[test]
fn demo_gpu_content_reuses_static_atlas_payload() {
    let first = demo_gpu_content();
    let second = demo_gpu_content();

    let (
        GpuSurfaceContent::RgbaAtlas { atlas: first, .. },
        GpuSurfaceContent::RgbaAtlas { atlas: second, .. },
    ) = (&first, &second)
    else {
        panic!("demo content should remain an atlas-backed GPU surface");
    };
    assert!(
        Arc::ptr_eq(first, second),
        "view reprojection should reuse the static GPU atlas instead of rebuilding pixel data"
    );
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

fn selection_overlay<Bridge, Message>(
    runtime: &SurfaceRuntime<Bridge, Message>,
) -> &SelectionOverlay
where
    Bridge: RuntimeBridge<Message>,
{
    runtime
        .surface()
        .find_widget(11)
        .expect("selection overlay should exist")
        .widget()
        .as_any()
        .downcast_ref::<SelectionOverlay>()
        .expect("widget should be selection overlay")
}

fn text_widget<Bridge, Message>(
    runtime: &SurfaceRuntime<Bridge, Message>,
    id: WidgetId,
) -> &TextWidget
where
    Bridge: RuntimeBridge<Message>,
{
    runtime
        .surface()
        .find_widget(id)
        .expect("text widget should exist")
        .widget()
        .as_any()
        .downcast_ref::<TextWidget>()
        .expect("widget should be text")
}
