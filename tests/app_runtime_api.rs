//! App-runtime API coverage for effects, startup commands, and repaint planning.

use radiant::{
    app,
    gui::{repaint::RepaintSignal, types::Vector2},
    runtime::{Command, RuntimeBridge, SurfaceChild, SurfaceNode, SurfaceRuntime, UiSurface},
    widgets::{TextInputWidget, TextWidget, WidgetSizing},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
enum DemoMessage {
    Increment,
}

#[derive(Default)]
struct DemoState {
    count: usize,
    name: String,
}

struct CountingRepaintSignal {
    called: Arc<AtomicBool>,
}

impl RepaintSignal for CountingRepaintSignal {
    fn request_repaint(&self) {
        self.called.store(true, Ordering::Release);
    }
}

#[derive(Default)]
struct PaintOnlyBridge {
    count: usize,
    project_count: usize,
}

impl RuntimeBridge<DemoMessage> for PaintOnlyBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        self.project_count += 1;
        Arc::new(UiSurface::new(SurfaceNode::static_widget(TextWidget::new(
            10,
            format!("PaintOnly ({})", self.count),
            WidgetSizing::fixed(Vector2::new(140.0, 20.0)).with_baseline(14.0),
        ))))
    }

    fn update(&mut self, message: DemoMessage) -> Command<DemoMessage> {
        if matches!(message, DemoMessage::Increment) {
            self.count += 1;
        }
        Command::request_paint_only()
    }
}

fn text_value<Message>(surface: &UiSurface<Message>, widget_id: u64) -> String {
    surface
        .find_widget(widget_id)
        .expect("widget exists")
        .widget_object()
        .as_any()
        .downcast_ref::<TextWidget>()
        .expect("widget is text")
        .text
        .clone()
}

#[test]
fn app_startup_commands_use_full_runtime_dispatch() {
    let bridge = app(DemoState::default())
        .view(|state: &mut DemoState| {
            UiSurface::new(SurfaceNode::row(
                1,
                8.0,
                vec![
                    SurfaceChild::fill(SurfaceNode::static_widget(TextWidget::new(
                        10,
                        format!("Startup ({})", state.count),
                        WidgetSizing::fixed(Vector2::new(140.0, 20.0)).with_baseline(14.0),
                    ))),
                    SurfaceChild::fill(SurfaceNode::static_widget(TextInputWidget::new(
                        11,
                        state.name.clone(),
                        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
                    ))),
                ],
            ))
        })
        .on_startup(|state, context| {
            state.name = String::from("ready");
            context.focus(11);
            context.request_repaint();
            context.after(Duration::from_millis(1), DemoMessage::Increment);
            context.spawn(
                "startup-increment",
                || DemoMessage::Increment,
                |message| message,
            );
        })
        .update_with(|state, message, _context| {
            if matches!(message, DemoMessage::Increment) {
                state.count += 1;
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(300.0, 48.0));
    let repaint_called = Arc::new(AtomicBool::new(false));
    runtime
        .bridge_mut()
        .install_repaint_signal(Arc::new(CountingRepaintSignal {
            called: Arc::clone(&repaint_called),
        }));

    let startup = runtime.drain_runtime_messages();
    assert!(startup.repaint_requested);
    assert_eq!(runtime.focused_widget(), Some(11));
    assert!(repaint_called.load(Ordering::Acquire));

    std::thread::sleep(Duration::from_millis(20));
    let drained = runtime.drain_runtime_messages();
    assert_eq!(drained.messages_dispatched, 2);
    assert_eq!(text_value(runtime.surface(), 10), "Startup (2)");
}

#[test]
fn app_runtime_effects_stop_after_runtime_exit() {
    let bridge = app(DemoState::default())
        .view(|state: &mut DemoState| {
            UiSurface::new(SurfaceNode::static_widget(TextWidget::new(
                10,
                format!("Effects ({})", state.count),
                WidgetSizing::fixed(Vector2::new(140.0, 20.0)).with_baseline(14.0),
            )))
        })
        .subscriptions(|_| {
            radiant::prelude::Subscription::interval("fast", Duration::from_millis(1), || {
                DemoMessage::Increment
            })
        })
        .update_with(|state, message, _context| {
            if matches!(message, DemoMessage::Increment) {
                state.count += 1;
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));
    runtime
        .bridge_mut()
        .install_repaint_signal(Arc::new(CountingRepaintSignal {
            called: Arc::new(AtomicBool::new(false)),
        }));

    std::thread::sleep(Duration::from_millis(15));
    let active = runtime.drain_runtime_messages();
    assert!(active.messages_dispatched > 0);

    let _ = runtime.bridge_mut().on_runtime_exit();
    let delayed = runtime.execute_command(Command::after(
        Duration::from_millis(5),
        DemoMessage::Increment,
    ));
    assert!(!delayed.repaint_requested);
    std::thread::sleep(Duration::from_millis(20));

    let stopped = runtime.drain_runtime_messages();
    assert_eq!(stopped.messages_dispatched, 0);
}

#[test]
fn active_animation_frame_messages_are_coalesced_until_drained() {
    let bridge = app(DemoState::default())
        .view(|state: &mut DemoState| {
            UiSurface::new(SurfaceNode::static_widget(TextWidget::new(
                10,
                format!("Frame ({})", state.count),
                WidgetSizing::fixed(Vector2::new(140.0, 20.0)).with_baseline(14.0),
            )))
        })
        .animation(|_| true)
        .on_frame(|| DemoMessage::Increment)
        .update_with(|state, message, _context| {
            if matches!(message, DemoMessage::Increment) {
                state.count += 1;
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));

    assert!(runtime.bridge_mut().needs_animation());
    assert!(runtime.bridge_mut().needs_animation());
    let drained = runtime.drain_runtime_messages();
    assert_eq!(drained.messages_dispatched, 1);
    assert_eq!(text_value(runtime.surface(), 10), "Frame (1)");

    assert!(runtime.bridge_mut().needs_animation());
    let drained = runtime.drain_runtime_messages();
    assert_eq!(drained.messages_dispatched, 1);
}

#[test]
fn paint_only_command_skips_surface_reprojection() {
    let bridge = PaintOnlyBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));

    assert_eq!(runtime.bridge().project_count, 1);
    assert_eq!(text_value(runtime.surface(), 10), "PaintOnly (0)");

    let outcome = runtime.dispatch_message(DemoMessage::Increment);

    assert!(outcome.repaint_requested);
    assert!(!outcome.surface_refresh_requested);
    assert_eq!(runtime.bridge().count, 1);
    assert_eq!(runtime.bridge().project_count, 1);
    assert_eq!(text_value(runtime.surface(), 10), "PaintOnly (0)");
}
