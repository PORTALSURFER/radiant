//! Public API coverage for the generic `radiant::runtime` surface.

use radiant::prelude::IntoView;
use radiant::{
    gui::{
        focus::FocusSurface,
        input::{KeyCode, KeyPress},
        shortcuts::ShortcutResolution,
        types::{ImageRgba, Rgba8},
    },
    layout::{Point, Rect, Vector2, layout_tree},
    runtime::{
        Command, Element, Event, FocusTraversal, GpuSurfaceCapabilities, GpuSurfaceContent,
        GpuSurfaceLineStyle, GpuSurfaceOverlay, GpuSurfaceRuntimeOverlays, PaintPrimitive,
        Renderer, RuntimeBridge, SurfaceChild, SurfaceNode, SurfacePaintPlan, SurfaceRuntime,
        UiSurface, View, WidgetMessageMapper, declarative_command_runtime_bridge,
        declarative_runtime_bridge,
    },
    theme::ThemeTokens,
    widgets::{
        ButtonWidget, CanvasMessage, DragHandleMessage, DragHandleWidget, GpuSurfaceWidget,
        PointerButton, RetainedSurfaceDescriptor, TextEditCommand, TextInputWidget, TextWidget,
        Widget, WidgetInput, WidgetKey, WidgetProminence, WidgetSizing, WidgetState, WidgetStyle,
        WidgetTone, resolve_widget_visual_tokens,
    },
};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq)]
enum DemoMessage {
    Increment,
    Rename(String),
    CanvasInput(WidgetInput),
}

#[derive(Default)]
struct DemoState {
    count: usize,
    name: String,
}

fn widget_ref<'a, T, Message>(surface: &'a UiSurface<Message>, id: u64, expected: &str) -> &'a T
where
    T: Widget + 'static,
{
    surface
        .find_widget(id)
        .unwrap_or_else(|| panic!("expected {expected} widget {id} to exist"))
        .widget()
        .as_any()
        .downcast_ref::<T>()
        .unwrap_or_else(|| panic!("expected widget {id} to be {expected}"))
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

#[cfg(test)]
#[path = "runtime_surface_public_api/interaction.rs"]
mod interaction;
#[cfg(test)]
#[path = "runtime_surface_public_api/paint_projection.rs"]
mod paint_projection;
#[cfg(test)]
#[path = "runtime_surface_public_api/pointer_motion.rs"]
mod pointer_motion;
#[cfg(test)]
#[path = "runtime_surface_public_api/virtual_scroll.rs"]
mod virtual_scroll;

#[test]
fn surface_runtime_hit_testing_prefers_topmost_declarative_widget() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        |_state: &mut DemoState| {
            Arc::new(UiSurface::new(SurfaceNode::stack(
                70,
                vec![
                    SurfaceChild::fill(SurfaceNode::button(
                        80,
                        "Bottom",
                        WidgetSizing::fixed(Vector2::new(120.0, 40.0)),
                        DemoMessage::Increment,
                    )),
                    SurfaceChild::fill(SurfaceNode::button(
                        90,
                        "Top",
                        WidgetSizing::fixed(Vector2::new(120.0, 40.0)),
                        DemoMessage::Rename(String::from("top")),
                    )),
                ],
            )))
        },
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let runtime = SurfaceRuntime::new(bridge, Vector2::new(140.0, 60.0));

    assert_eq!(runtime.widget_at(Point::new(16.0, 16.0)), Some(90));
}

#[test]
fn surface_runtime_hit_testing_skips_passive_widget_leaves() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        |_state: &mut DemoState| {
            Arc::new(UiSurface::new(SurfaceNode::stack(
                70,
                vec![
                    SurfaceChild::fill(SurfaceNode::button(
                        80,
                        "Interactive",
                        WidgetSizing::fixed(Vector2::new(120.0, 40.0)),
                        DemoMessage::Increment,
                    )),
                    SurfaceChild::fill(SurfaceNode::static_widget(TextWidget::new(
                        90,
                        "Passive label",
                        WidgetSizing::fixed(Vector2::new(120.0, 40.0)),
                    ))),
                ],
            )))
        },
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let runtime = SurfaceRuntime::new(bridge, Vector2::new(140.0, 60.0));

    assert_eq!(runtime.widget_at(Point::new(16.0, 16.0)), Some(80));
}

#[test]
fn surface_runtime_executes_command_messages_and_repaint_requests() {
    let bridge = CommandDemoBridge {
        state: DemoState::default(),
    };
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    let outcome = runtime.dispatch_message(CommandDemoMessage::Start);

    assert_eq!(outcome.messages_dispatched, 3);
    assert!(outcome.repaint_requested);
    assert!(outcome.surface_refresh_requested);
    assert!(runtime.repaint_requested());
    assert!(runtime.take_repaint_requested());
    assert!(!runtime.repaint_requested());

    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "Commands (1)"
    );
    assert_eq!(
        widget_ref::<TextInputWidget, _>(runtime.surface(), 12, "text input")
            .state
            .value,
        "Commands"
    );
}

#[test]
fn surface_runtime_executes_focus_exit_and_deferred_commands() {
    let bridge = RuntimeCommandBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(160.0, 80.0));

    let focus = runtime.execute_command(Command::focus(11));
    assert!(!focus.exit_requested);
    assert_eq!(runtime.focused_widget(), Some(11));

    let deferred = runtime.execute_command(Command::after(
        Duration::from_millis(1),
        DemoMessage::Increment,
    ));
    assert!(deferred.repaint_requested);
    let drained = drain_until_messages(&mut runtime, 1);
    assert_eq!(drained.messages_dispatched, 1);
    assert_eq!(runtime.bridge().count, 1);

    let performed = runtime.execute_command(Command::perform(
        "increment",
        || DemoMessage::Increment,
        |message| message,
    ));
    assert!(performed.repaint_requested);
    let drained = drain_until_messages(&mut runtime, 1);
    assert_eq!(drained.messages_dispatched, 1);
    assert_eq!(runtime.bridge().count, 2);

    let exit = runtime.execute_command(Command::exit());
    assert!(exit.exit_requested);
    assert!(runtime.take_exit_requested());
}

fn project_surface(state: &mut DemoState) -> Arc<UiSurface<DemoMessage>> {
    let title = TextWidget::new(
        10,
        format!("{} ({})", display_name(state), state.count),
        WidgetSizing::fixed(Vector2::new(140.0, 20.0)).with_baseline(14.0),
    );
    let button = ButtonWidget::new(
        11,
        "Increment",
        WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
    );
    Arc::new(UiSurface::new(SurfaceNode::row(
        1,
        8.0,
        vec![
            SurfaceChild::fill(SurfaceNode::static_widget(title)),
            SurfaceChild::fill(SurfaceNode::widget(
                button,
                WidgetMessageMapper::button(|_| DemoMessage::Increment),
            )),
            SurfaceChild::fill(SurfaceNode::text_input(
                12,
                state.name.clone(),
                WidgetSizing::new(Vector2::new(120.0, 28.0), Vector2::new(180.0, 28.0)),
                DemoMessage::Rename,
            )),
        ],
    )))
}

fn display_name(state: &DemoState) -> &str {
    if state.name.is_empty() {
        "Untitled"
    } else {
        &state.name
    }
}

fn button_hovered(surface: &UiSurface<DemoMessage>, widget_id: u64) -> bool {
    widget_ref::<ButtonWidget, _>(surface, widget_id, "button")
        .common
        .state
        .hovered
}

enum CommandDemoMessage {
    Start,
    Increment,
    Rename(String),
}

struct CommandDemoBridge {
    state: DemoState,
}

#[derive(Default)]
struct RuntimeCommandBridge {
    count: usize,
    pending: Arc<std::sync::Mutex<Vec<DemoMessage>>>,
}

#[derive(Default)]
struct ShortcutDemoBridge {
    state: DemoState,
}

impl RuntimeBridge<DemoMessage> for ShortcutDemoBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        project_surface(&mut self.state)
    }

    fn update(&mut self, message: DemoMessage) -> Command<DemoMessage> {
        match message {
            DemoMessage::Increment => self.state.count += 1,
            DemoMessage::Rename(name) => self.state.name = name,
            DemoMessage::CanvasInput(_) => {}
        }
        Command::none()
    }

    fn resolve_key_press(
        &mut self,
        _pending_chord: Option<KeyPress>,
        press: KeyPress,
        _focus: FocusSurface,
    ) -> ShortcutResolution<DemoMessage> {
        if press == KeyPress::with_command(KeyCode::I) {
            return ShortcutResolution {
                action: Some(DemoMessage::Increment),
                handled: true,
                pending_chord: None,
            };
        }
        ShortcutResolution::unhandled()
    }
}

impl RuntimeBridge<CommandDemoMessage> for CommandDemoBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<CommandDemoMessage>> {
        project_demo_surface(&mut self.state)
    }

    fn update(&mut self, message: CommandDemoMessage) -> Command<CommandDemoMessage> {
        match message {
            CommandDemoMessage::Start => Command::batch([
                Command::message(CommandDemoMessage::Rename(String::from("Commands"))),
                Command::request_repaint(),
                Command::message(CommandDemoMessage::Increment),
                Command::request_paint_only(),
            ]),
            CommandDemoMessage::Increment => {
                self.state.count += 1;
                Command::none()
            }
            CommandDemoMessage::Rename(name) => {
                self.state.name = name;
                Command::none()
            }
        }
    }
}

impl RuntimeBridge<DemoMessage> for RuntimeCommandBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        Arc::new(UiSurface::new(SurfaceNode::row(
            1,
            8.0,
            vec![SurfaceChild::fill(SurfaceNode::static_widget(
                ButtonWidget::new(11, "Focus", WidgetSizing::fixed(Vector2::new(80.0, 32.0))),
            ))],
        )))
    }

    fn update(&mut self, message: DemoMessage) -> Command<DemoMessage> {
        if matches!(message, DemoMessage::Increment) {
            self.count += 1;
        }
        Command::none()
    }

    fn schedule_message(&mut self, delay: Duration, message: DemoMessage) -> bool {
        let pending = Arc::clone(&self.pending);
        std::thread::spawn(move || {
            std::thread::sleep(delay);
            pending
                .lock()
                .expect("pending messages poisoned")
                .push(message);
        });
        true
    }

    fn spawn_message_task(
        &mut self,
        _name: &'static str,
        work: Box<dyn FnOnce() -> DemoMessage + Send + 'static>,
    ) -> bool {
        let pending = Arc::clone(&self.pending);
        std::thread::spawn(move || {
            pending
                .lock()
                .expect("pending messages poisoned")
                .push(work());
        });
        true
    }

    fn take_runtime_messages(&mut self) -> Vec<DemoMessage> {
        std::mem::take(&mut *self.pending.lock().expect("pending messages poisoned"))
    }
}

fn project_demo_surface(state: &mut DemoState) -> Arc<UiSurface<CommandDemoMessage>> {
    let title = TextWidget::new(
        10,
        format!("{} ({})", display_name(state), state.count),
        WidgetSizing::fixed(Vector2::new(140.0, 20.0)).with_baseline(14.0),
    );
    let button = ButtonWidget::new(11, "Run", WidgetSizing::fixed(Vector2::new(96.0, 28.0)));
    let input = TextInputWidget::new(
        12,
        state.name.clone(),
        WidgetSizing::new(Vector2::new(120.0, 28.0), Vector2::new(180.0, 28.0)),
    );

    Arc::new(UiSurface::new(SurfaceNode::row(
        1,
        8.0,
        vec![
            SurfaceChild::fill(SurfaceNode::static_widget(title)),
            SurfaceChild::fill(SurfaceNode::widget(
                button,
                WidgetMessageMapper::button(|_| CommandDemoMessage::Start),
            )),
            SurfaceChild::fill(SurfaceNode::widget(input, WidgetMessageMapper::none())),
        ],
    )))
}
