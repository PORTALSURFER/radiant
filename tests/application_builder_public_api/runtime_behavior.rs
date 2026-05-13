use super::{DemoMessage, DemoState, widget_ref};
use radiant::{
    layout::Vector2,
    runtime::{Command, PaintFillRect, PaintPrimitive, RuntimeBridge, SurfaceRuntime},
    widgets::{ButtonMessage, TextWidget},
};
use std::{
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, PartialEq, Eq)]
enum LoadingMessage {
    Start,
    Loaded(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum FocusMessage {
    FocusName,
}

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
    assert!(!runtime.bridge_mut().queue_animation_frame());

    let drained = runtime.drain_runtime_messages();
    assert_eq!(drained.messages_dispatched, 0);
    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "Frame 0"
    );
}

#[test]
fn application_builder_background_spawn_routes_worker_result() {
    use radiant::prelude as ui;

    let bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::column([
                ui::text(format!("Loaded: {}", state.name))
                    .id(10)
                    .height(24.0),
                ui::button("Load")
                    .message(LoadingMessage::Start)
                    .id(11)
                    .height(28.0),
            ])
        })
        .update_with(|state, message, context| match message {
            LoadingMessage::Start => {
                state.name = "loading".to_string();
                context.spawn(
                    "test-loader",
                    || "ready".to_string(),
                    LoadingMessage::Loaded,
                );
                context.request_repaint();
            }
            LoadingMessage::Loaded(value) => {
                state.name = value;
                context.request_repaint();
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 80.0));
    let start = runtime
        .surface()
        .dispatch_widget_output(
            11,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        )
        .expect("load button should emit a start message");

    let started = runtime.dispatch_message(start);
    assert!(started.repaint_requested);
    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "Loaded: loading"
    );

    let deadline = Instant::now() + Duration::from_secs(1);
    let finished = loop {
        let finished = runtime.drain_runtime_messages();
        if finished.messages_dispatched > 0 || Instant::now() >= deadline {
            break finished;
        }
        thread::sleep(Duration::from_millis(1));
    };
    assert_eq!(finished.messages_dispatched, 1);
    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "Loaded: ready"
    );
}

#[test]
fn application_builder_update_context_can_move_keyboard_focus() {
    use radiant::prelude as ui;

    let bridge = ui::app(DemoState::default())
        .view(|state| {
            ui::column([
                ui::text_input(state.name.clone())
                    .message(|_| FocusMessage::FocusName)
                    .id(10),
                ui::button("Focus name")
                    .message(FocusMessage::FocusName)
                    .id(11),
                ui::text(format!("Name: {}", state.name))
                    .id(12)
                    .height(24.0),
            ])
        })
        .update_with(|state, message, context| match message {
            FocusMessage::FocusName => {
                state.name = String::from("focused");
                context.focus(10);
                context.request_repaint();
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 120.0));
    let focus = runtime
        .surface()
        .dispatch_widget_output(
            11,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        )
        .expect("button should emit focus message");
    let outcome = runtime.dispatch_message(focus);

    assert!(outcome.repaint_requested);
    assert_eq!(runtime.focused_widget(), Some(10));
    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 12, "status").text,
        "Name: focused"
    );
}

#[test]
fn stateful_app_builder_projects_updates_and_preserves_commands() {
    use radiant::prelude as ui;

    let mut bridge = ui::app(DemoState::default())
        .title("Counter")
        .size(320, 120)
        .view(|state| {
            ui::column([
                ui::text(format!("Count: {}", state.count)),
                ui::button("Increment").message(DemoMessage::Increment),
            ])
        })
        .update_command(|state, message| match message {
            DemoMessage::Increment => {
                state.count += 1;
                Command::request_repaint()
            }
        })
        .into_bridge();

    let before = bridge.project_surface();
    let increment = before
        .dispatch_widget_output(
            3,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        )
        .expect("generated button should route through the same surface mapper");

    let command = bridge.update(increment);

    assert!(command.requests_repaint());
    let after = bridge.project_surface();
    assert_eq!(
        widget_ref::<TextWidget, _>(&after, 2, "text").text,
        "Count: 1"
    );
}
