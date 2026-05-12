//! App-runtime API coverage for effects, startup commands, and repaint planning.

use radiant::{
    app,
    gui::{
        focus::FocusSurface,
        input::{KeyCode, KeyPress},
        repaint::RepaintSignal,
        shortcuts::ShortcutResolution,
        types::Vector2,
    },
    layout::{Constraints, SizeModeCross, SizeModeMain, SlotParams},
    runtime::{Command, RuntimeBridge, SurfaceChild, SurfaceNode, SurfaceRuntime, UiSurface},
    widgets::{ButtonWidget, TextInputWidget, TextWidget, WidgetInput, WidgetKey, WidgetSizing},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

#[path = "app_runtime_api/gpu_surface.rs"]
mod gpu_surface;
#[path = "app_runtime_api/scroll_hooks.rs"]
mod scroll_hooks;

#[derive(Clone, Debug, PartialEq)]
enum DemoMessage {
    Increment,
    GpuInput(WidgetInput),
}

#[derive(Default)]
struct DemoState {
    count: usize,
    name: String,
    last_scroll_y: f32,
}

struct CountingRepaintSignal {
    called: Arc<AtomicBool>,
}

#[derive(Default)]
struct DrainIntoBridge {
    commands: Vec<Command<DemoMessage>>,
    messages: Vec<DemoMessage>,
    drained_commands_into: bool,
    drained_messages_into: bool,
}

impl RepaintSignal for CountingRepaintSignal {
    fn request_repaint(&self) {
        self.called.store(true, Ordering::Release);
    }
}

impl RuntimeBridge<DemoMessage> for DrainIntoBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        Arc::new(UiSurface::new(SurfaceNode::static_widget(TextWidget::new(
            10,
            "DrainInto",
            WidgetSizing::fixed(Vector2::new(120.0, 20.0)).with_baseline(14.0),
        ))))
    }

    fn drain_runtime_commands_into(&mut self, commands: &mut Vec<Command<DemoMessage>>) {
        self.drained_commands_into = true;
        commands.append(&mut self.commands);
    }

    fn drain_runtime_messages_into(&mut self, messages: &mut Vec<DemoMessage>) {
        self.drained_messages_into = true;
        messages.append(&mut self.messages);
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
        .to_string()
}

fn drain_until_messages<Bridge>(
    runtime: &mut SurfaceRuntime<Bridge, DemoMessage>,
    min_messages: usize,
) -> radiant::runtime::CommandOutcome
where
    Bridge: RuntimeBridge<DemoMessage>,
{
    let deadline = Instant::now() + Duration::from_secs(1);
    let mut drained = radiant::runtime::CommandOutcome::default();
    loop {
        let outcome = runtime.drain_runtime_messages();
        drained.messages_dispatched += outcome.messages_dispatched;
        drained.repaint_requested |= outcome.repaint_requested;
        drained.surface_refresh_requested |= outcome.surface_refresh_requested;
        drained.exit_requested |= outcome.exit_requested;
        if drained.messages_dispatched >= min_messages || Instant::now() >= deadline {
            return drained;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
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

    let drained = drain_until_messages(&mut runtime, 2);
    assert_eq!(drained.messages_dispatched, 2);
    assert_eq!(text_value(runtime.surface(), 10), "Startup (2)");
}

#[test]
fn app_startup_runs_once_when_repaint_signal_is_reinstalled() {
    let mut bridge = app(DemoState::default())
        .view(|state: &mut DemoState| {
            UiSurface::new(SurfaceNode::static_widget(TextWidget::new(
                10,
                format!("Startup runs: {}", state.count),
                WidgetSizing::fixed(Vector2::new(160.0, 20.0)).with_baseline(14.0),
            )))
        })
        .on_startup(|state, _context| {
            state.count += 1;
        })
        .update_with(|_state, _message: DemoMessage, _context| {})
        .into_bridge();

    bridge.install_repaint_signal(Arc::new(CountingRepaintSignal {
        called: Arc::new(AtomicBool::new(false)),
    }));
    bridge.install_repaint_signal(Arc::new(CountingRepaintSignal {
        called: Arc::new(AtomicBool::new(false)),
    }));

    let surface = bridge.project_surface();

    assert_eq!(text_value(&surface, 10), "Startup runs: 1");
}

fn intrinsic_slot() -> SlotParams {
    SlotParams {
        size_main: SizeModeMain::Intrinsic,
        size_cross: SizeModeCross::Fill,
        constraints: Constraints::unconstrained(),
        margin: Default::default(),
        align_cross_override: None,
        allow_fixed_compress: false,
    }
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

    let active = drain_until_messages(&mut runtime, 1);
    assert!(active.messages_dispatched > 0);

    let _ = runtime.bridge_mut().on_runtime_exit();
    let delayed = runtime.execute_command(Command::after(
        Duration::from_millis(5),
        DemoMessage::Increment,
    ));
    assert!(!delayed.repaint_requested);
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
    assert!(runtime.bridge_mut().queue_animation_frame());
    assert!(!runtime.bridge_mut().queue_animation_frame());
    let drained = runtime.drain_runtime_messages();
    assert_eq!(drained.messages_dispatched, 1);
    assert_eq!(text_value(runtime.surface(), 10), "Frame (1)");

    assert!(runtime.bridge_mut().needs_animation());
    assert!(runtime.bridge_mut().queue_animation_frame());
    let drained = runtime.drain_runtime_messages();
    assert_eq!(drained.messages_dispatched, 1);
}

#[test]
fn polling_animation_activity_does_not_queue_frame_messages() {
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

    assert_eq!(drained.messages_dispatched, 0);
    assert_eq!(text_value(runtime.surface(), 10), "Frame (0)");
}

#[test]
fn surface_runtime_uses_bridge_drain_into_hooks_for_runtime_work() {
    let bridge = DrainIntoBridge {
        commands: vec![Command::request_repaint()],
        messages: vec![DemoMessage::Increment],
        ..DrainIntoBridge::default()
    };
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));

    let drained = runtime.drain_runtime_messages();

    assert_eq!(drained.messages_dispatched, 1);
    assert!(drained.repaint_requested);
    assert!(runtime.bridge().drained_commands_into);
    assert!(runtime.bridge().drained_messages_into);
}

fn button_label<Message>(surface: &UiSurface<Message>, widget_id: u64) -> String {
    surface
        .find_widget(widget_id)
        .expect("widget exists")
        .widget_object()
        .as_any()
        .downcast_ref::<ButtonWidget>()
        .expect("widget is button")
        .props
        .label
        .to_string()
}

#[test]
fn app_shortcuts_dispatch_messages_before_focused_widget_key_routing() {
    let bridge = app(DemoState::default())
        .view(|state: &mut DemoState| {
            radiant::prelude::button(format!("Count {}", state.count))
                .message(DemoMessage::Increment)
                .id(10)
        })
        .shortcuts(|_, _, press, _| {
            if press == KeyPress::with_command(KeyCode::I) {
                ShortcutResolution::action(DemoMessage::Increment)
            } else if press == KeyPress::new(KeyCode::Enter) {
                ShortcutResolution::handled()
            } else {
                ShortcutResolution::unhandled()
            }
        })
        .update_with(|state, message, _context| {
            if matches!(message, DemoMessage::Increment) {
                state.count += 1;
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));

    assert!(runtime.dispatch_key_press(
        KeyPress::with_command(KeyCode::I),
        None,
        FocusSurface::None
    ));
    assert_eq!(button_label(runtime.surface(), 10), "Count 1");

    assert!(runtime.focus_widget(10));
    assert!(runtime.dispatch_key_press(
        KeyPress::new(KeyCode::Enter),
        Some(WidgetKey::Enter),
        FocusSurface::None
    ));
    assert_eq!(button_label(runtime.surface(), 10), "Count 1");

    assert!(runtime.dispatch_key_press(
        KeyPress::new(KeyCode::Space),
        Some(WidgetKey::Space),
        FocusSurface::None
    ));
    assert_eq!(button_label(runtime.surface(), 10), "Count 2");
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
