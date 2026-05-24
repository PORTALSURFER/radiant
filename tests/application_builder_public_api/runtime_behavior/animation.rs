use super::*;

#[test]
fn application_builder_animation_frames_route_through_public_app_path() {
    use radiant::prelude as ui;

    let bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::text(format!("Frame {}", state.count))
                .id(10)
                .height(24.0)
        })
        .animation(|_| true)
        .on_frame(|| DemoMessage::Increment)
        .update(|state, message| match message {
            DemoMessage::Increment => state.count += 1,
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 48.0));

    assert!(runtime.bridge_mut().needs_animation());
    let activity = runtime.bridge_mut().animation_activity();
    assert!(activity.needs_animation());
    assert!(activity.needs_frame_message());
    assert!(runtime.bridge_mut().needs_animation());
    assert!(runtime.bridge_mut().queue_animation_frame());
    assert!(!runtime.bridge_mut().queue_animation_frame());
    let drained = runtime.drain_runtime_messages();

    assert_eq!(drained.messages_dispatched, 1);
    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "Frame 1"
    );
}

#[test]
fn animated_transient_overlay_uses_paint_only_frame_activity() {
    use radiant::prelude as ui;

    let bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::text(format!("Frame {}", state.count))
                .id(10)
                .height(24.0)
        })
        .on_frame(|| DemoMessage::Increment)
        .animated_transient_overlay(
            |_| true,
            |state, context, primitives| {
                primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                    widget_id: 10,
                    rect: ui::Rect::from_min_size(
                        ui::Point::new(context.animation_time.as_secs_f32(), 0.0),
                        ui::Vector2::new(4.0, 4.0),
                    ),
                    color: ui::Rgba8 {
                        r: state.count as u8,
                        g: 128,
                        b: 255,
                        a: 255,
                    },
                }));
            },
        )
        .update(|state, message| match message {
            DemoMessage::Increment => state.count += 1,
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 48.0));

    assert!(runtime.bridge_mut().needs_animation());
    let activity = runtime.bridge_mut().animation_activity();
    assert!(activity.needs_animation());
    assert!(!activity.needs_frame_message());
    assert_eq!(activity.target_fps(), None);
    assert!(!runtime.bridge_mut().queue_animation_frame());

    let drained = runtime.drain_runtime_messages();
    assert_eq!(drained.messages_dispatched, 0);
    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "Frame 0"
    );
}

#[test]
fn animated_transient_overlay_can_cap_paint_only_frame_rate() {
    use radiant::prelude as ui;

    let bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::text(format!("Frame {}", state.count))
                .id(10)
                .height(24.0)
        })
        .animated_transient_overlay_at(
            24,
            |_| true,
            |_state, _context, primitives| {
                primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                    widget_id: 10,
                    rect: ui::Rect::from_min_size(
                        ui::Point::new(0.0, 0.0),
                        ui::Vector2::new(4.0, 4.0),
                    ),
                    color: ui::Rgba8 {
                        r: 255,
                        g: 128,
                        b: 255,
                        a: 255,
                    },
                }));
            },
        )
        .update(|state, message| match message {
            DemoMessage::Increment => state.count += 1,
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 48.0));

    let activity = runtime.bridge_mut().animation_activity();
    assert!(activity.needs_animation());
    assert!(!activity.needs_frame_message());
    assert_eq!(activity.target_fps(), Some(24));
    assert!(!runtime.bridge_mut().queue_animation_frame());
}

#[test]
fn app_frame_animation_keeps_native_cadence_when_overlay_is_capped() {
    use radiant::prelude as ui;

    let bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::text(format!("Frame {}", state.count))
                .id(10)
                .height(24.0)
        })
        .animation(|_| true)
        .on_frame(|| DemoMessage::Increment)
        .animated_transient_overlay_at(24, |_| true, |_state, _context, _primitives| {})
        .update(|state, message| match message {
            DemoMessage::Increment => state.count += 1,
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 48.0));

    let activity = runtime.bridge_mut().animation_activity();
    assert!(activity.needs_animation());
    assert!(activity.needs_frame_message());
    assert_eq!(activity.target_fps(), None);
}
