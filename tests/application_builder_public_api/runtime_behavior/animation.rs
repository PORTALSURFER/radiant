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

#[test]
fn presentation_frame_clock_queues_frame_messages() {
    use radiant::prelude as ui;

    let bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::text(format!("Frame {}", state.count))
                .id(10)
                .height(24.0)
        })
        .presentation(
            ui::presentation()
                .frame_clock(ui::FrameClock::message(DemoMessage::Increment).when(|_| true)),
        )
        .handle_message(|state, message, _context| match message {
            DemoMessage::Increment => state.count += 1,
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 48.0));

    let activity = runtime.bridge_mut().animation_activity();
    assert!(activity.needs_animation());
    assert!(activity.needs_frame_message());
    assert_eq!(activity.target_fps(), None);
    assert!(runtime.bridge_mut().queue_animation_frame());

    let drained = runtime.drain_runtime_messages();

    assert_eq!(drained.messages_dispatched, 1);
    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "Frame 1"
    );
}

#[test]
fn presentation_frame_clock_can_cap_frame_message_rate() {
    use radiant::prelude as ui;

    let bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::text(format!("Frame {}", state.count))
                .id(10)
                .height(24.0)
        })
        .presentation(
            ui::Presentation::new().frame_clock(
                ui::FrameClock::message_with(|| DemoMessage::Increment)
                    .when(|_| true)
                    .fps(30),
            ),
        )
        .handle_message(|state, message, _context| match message {
            DemoMessage::Increment => state.count += 1,
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 48.0));

    let activity = runtime.bridge_mut().animation_activity();

    assert!(activity.needs_animation());
    assert!(activity.needs_frame_message());
    assert_eq!(activity.target_fps(), Some(30));
}

#[test]
fn presentation_transient_overlay_uses_paint_only_frame_activity() {
    use radiant::prelude as ui;

    let bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::text(format!("Frame {}", state.count))
                .id(10)
                .height(24.0)
        })
        .presentation(
            ui::Presentation::new().transient_overlay(
                ui::TransientOverlay::new(7_u64)
                    .paint_only()
                    .when(|_| true)
                    .paint(|state: &mut DemoState, context, primitives| {
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
                    }),
            ),
        )
        .update(|state, message| match message {
            DemoMessage::Increment => state.count += 1,
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 48.0));

    let activity = runtime.bridge_mut().animation_activity();

    assert!(activity.needs_animation());
    assert!(!activity.needs_frame_message());
    assert_eq!(activity.target_fps(), None);
    assert!(!runtime.bridge_mut().queue_animation_frame());
}

#[test]
fn presentation_transient_overlay_can_cap_paint_only_frame_rate() {
    use radiant::prelude as ui;

    let bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::text(format!("Frame {}", state.count))
                .id(10)
                .height(24.0)
        })
        .presentation(
            ui::Presentation::new().transient_overlay(
                ui::TransientOverlay::new(7_u64)
                    .paint_only()
                    .when(|_| true)
                    .fps(24)
                    .paint(|_state: &mut DemoState, _context, _primitives| {}),
            ),
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
}

#[test]
fn presentation_frame_clock_repaint_scope_requests_paint_only_after_frame_update() {
    use radiant::prelude as ui;

    let bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::text(format!("Frame {}", state.count))
                .id(10)
                .height(24.0)
        })
        .presentation(
            ui::Presentation::new().frame_clock(
                ui::FrameClock::message(DemoMessage::Increment)
                    .when(|_| true)
                    .repaint_scope(
                        |state: &mut DemoState| state.count,
                        |state, before| state.count == before + 1,
                    ),
            ),
        )
        .handle_message(|state, message, _context| match message {
            DemoMessage::Increment => state.count += 1,
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 48.0));

    assert!(
        runtime
            .bridge_mut()
            .animation_activity()
            .needs_frame_message()
    );
    assert!(runtime.bridge_mut().queue_animation_frame());
    let command = runtime.bridge_mut().update(DemoMessage::Increment);

    assert!(command.requests_paint_only());
}

#[test]
fn presentation_frame_clock_without_repaint_scope_requests_surface_after_frame_update() {
    use radiant::prelude as ui;
    use radiant::runtime::RepaintScope;

    let mut bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::text(format!("Frame {}", state.count))
                .id(10)
                .height(24.0)
        })
        .presentation(
            ui::Presentation::new()
                .frame_clock(ui::FrameClock::message(DemoMessage::Increment).when(|_| true)),
        )
        .handle_message(|state, message, _context| match message {
            DemoMessage::Increment => state.count += 1,
        })
        .into_bridge();

    assert!(bridge.animation_activity().needs_frame_message());
    assert!(bridge.queue_animation_frame());
    let command = bridge.update(DemoMessage::Increment);

    assert_eq!(command.repaint_scope(), Some(RepaintScope::Surface));
}

#[test]
fn frame_clock_origin_takes_precedence_over_ordinary_repaint_policy() {
    use radiant::prelude as ui;
    use radiant::runtime::RepaintScope;

    let mut bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::text(format!("Frame {}", state.count))
                .id(10)
                .height(24.0)
        })
        .presentation(
            ui::Presentation::new()
                .frame_clock(ui::FrameClock::message(DemoMessage::Increment).when(|_| true)),
        )
        .handle_message(|state, message, _context| match message {
            DemoMessage::Increment => state.count += 1,
        })
        .repaint_policy(ui::RepaintPolicy::none())
        .into_bridge();

    assert!(bridge.animation_activity().needs_frame_message());
    assert!(bridge.queue_animation_frame());
    let command = bridge.update(DemoMessage::Increment);

    assert_eq!(command.repaint_scope(), Some(RepaintScope::Surface));
}

#[test]
fn scene_frame_clock_queues_frame_message_when_active() {
    use radiant::prelude as ui;

    let bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::scene(
                ui::text(format!("Frame {}", state.count))
                    .id(10)
                    .height(24.0),
            )
            .frame_clock(
                ui::FrameClock::message(DemoMessage::Increment).when(|_state: &mut DemoState| true),
            )
            .into_view()
        })
        .handle_message(|state, message, _context| match message {
            DemoMessage::Increment => state.count += 1,
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 48.0));

    let activity = runtime.bridge_mut().animation_activity();
    assert!(activity.needs_animation());
    assert!(activity.needs_frame_message());
    assert!(runtime.bridge_mut().queue_animation_frame());

    let drained = runtime.drain_runtime_messages();

    assert_eq!(drained.messages_dispatched, 1);
    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "Frame 1"
    );
}

#[test]
fn scene_frame_clock_without_repaint_scope_requests_surface_after_frame_update() {
    use radiant::prelude as ui;
    use radiant::runtime::RepaintScope;

    let mut bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::scene(
                ui::text(format!("Frame {}", state.count))
                    .id(10)
                    .height(24.0),
            )
            .frame_clock(
                ui::FrameClock::message(DemoMessage::Increment).when(|_state: &mut DemoState| true),
            )
            .into_view()
        })
        .handle_message(|state, message, _context| match message {
            DemoMessage::Increment => state.count += 1,
        })
        .into_bridge();
    bridge.project_surface();

    assert!(bridge.animation_activity().needs_frame_message());
    assert!(bridge.queue_animation_frame());
    let command = bridge.update(DemoMessage::Increment);

    assert_eq!(command.repaint_scope(), Some(RepaintScope::Surface));
}

#[test]
fn scene_frame_clock_stays_idle_when_inactive() {
    use radiant::prelude as ui;

    let bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::scene(
                ui::text(format!("Frame {}", state.count))
                    .id(10)
                    .height(24.0),
            )
            .frame_clock(
                ui::FrameClock::message(DemoMessage::Increment)
                    .when(|_state: &mut DemoState| false),
            )
            .into_view()
        })
        .handle_message(|state, message, _context| match message {
            DemoMessage::Increment => state.count += 1,
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 48.0));

    let activity = runtime.bridge_mut().animation_activity();

    assert!(!activity.needs_animation());
    assert!(!activity.needs_frame_message());
    assert!(!runtime.bridge_mut().queue_animation_frame());
}

#[test]
fn scene_transient_overlay_requests_paint_only_frames() {
    use radiant::prelude as ui;

    let bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::scene(
                ui::text(format!("Frame {}", state.count))
                    .id(10)
                    .height(24.0),
            )
            .overlay(
                ui::TransientOverlay::new(7_u64)
                    .paint_only()
                    .when(|_| true)
                    .paint(|state: &mut DemoState, _context, primitives| {
                        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                            widget_id: 10,
                            rect: ui::Rect::from_size(4.0, 4.0),
                            color: ui::Rgba8::new(state.count as u8, 128, 255, 255),
                        }));
                    }),
            )
            .into_view()
        })
        .update(|state, message| match message {
            DemoMessage::Increment => state.count += 1,
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 48.0));

    let activity = runtime.bridge_mut().animation_activity();

    assert!(activity.needs_animation());
    assert!(!activity.needs_frame_message());
    assert_eq!(activity.target_fps(), None);
    assert!(!runtime.bridge_mut().queue_animation_frame());
    assert!(runtime.bridge_mut().has_transient_overlay_painter());
}

#[test]
fn scene_transient_overlay_respects_target_fps() {
    use radiant::prelude as ui;

    let bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::scene(
                ui::text(format!("Frame {}", state.count))
                    .id(10)
                    .height(24.0),
            )
            .overlay(
                ui::TransientOverlay::new(7_u64)
                    .paint_only()
                    .when(|_| true)
                    .fps(24)
                    .paint(|_state: &mut DemoState, _context, _primitives| {}),
            )
            .into_view()
        })
        .update(|state, message| match message {
            DemoMessage::Increment => state.count += 1,
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 48.0));

    let activity = runtime.bridge_mut().animation_activity();

    assert!(activity.needs_animation());
    assert!(!activity.needs_frame_message());
    assert_eq!(activity.target_fps(), Some(24));
}

#[test]
fn scene_presentation_merges_with_layer_projection_without_affecting_input() {
    use radiant::prelude as ui;

    let bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::scene(
                ui::button(format!("Base {}", state.count))
                    .message(DemoMessage::Increment)
                    .id(42),
            )
            .frame_clock(
                ui::FrameClock::message(DemoMessage::Increment).when(|_state: &mut DemoState| true),
            )
            .layer(radiant::Layer::tooltip(ui::text("Tip").height(20.0)))
            .into_view()
        })
        .handle_message(|state, message, _context| match message {
            DemoMessage::Increment => state.count += 1,
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 48.0));

    assert!(
        runtime
            .bridge_mut()
            .animation_activity()
            .needs_frame_message()
    );
    assert_eq!(runtime.widget_at(ui::Point::new(16.0, 12.0)), Some(42));
    assert!(runtime.bridge_mut().queue_animation_frame());
    let drained = runtime.drain_runtime_messages();

    assert_eq!(drained.messages_dispatched, 1);
}

#[test]
fn scene_presentation_preserves_base_widget_hit_testing() {
    use radiant::prelude as ui;

    let bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::scene(
                ui::button(format!("Base {}", state.count))
                    .message(DemoMessage::Increment)
                    .id(42)
                    .height(32.0),
            )
            .overlay(
                ui::TransientOverlay::new(7_u64)
                    .paint_only()
                    .when(|_| true)
                    .paint(|_state: &mut DemoState, _context, _primitives| {}),
            )
            .into_view()
        })
        .handle_message(|state, message, _context| match message {
            DemoMessage::Increment => state.count += 1,
        })
        .into_bridge();
    let runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 48.0));

    assert_eq!(runtime.widget_at(ui::Point::new(16.0, 12.0)), Some(42));
}
