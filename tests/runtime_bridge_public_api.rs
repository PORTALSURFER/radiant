//! Public API coverage for generic runtime bridge contracts.

use radiant::{
    gui::repaint::RepaintSignal,
    layout::Vector2,
    runtime::{
        App, Command, ResourceLoad, ResourceLoadState, ResourceSlot, RuntimeBridge, SurfaceChild,
        SurfaceNode, SurfaceRuntime, UiSurface, WidgetMessageMapper,
        declarative_command_runtime_bridge, declarative_runtime_bridge,
    },
    widgets::{
        ButtonMessage, ButtonWidget, TextInputMessage, TextInputWidget, TextWidget, Widget,
        WidgetSizing,
    },
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Debug, PartialEq)]
enum DemoMessage {
    Increment,
    Rename(String),
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

#[test]
fn generic_runtime_bridge_projects_and_reduces_host_defined_messages() {
    let mut bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
        },
    );

    let surface_before = bridge.project_surface();
    let rename = surface_before
        .dispatch_widget_output(
            12,
            radiant::widgets::WidgetOutput::typed(TextInputMessage::Changed {
                value: String::from("Projects"),
            }),
        )
        .expect("text input should emit a host-defined rename message");
    bridge.reduce_message(rename);

    let increment = bridge
        .project_surface()
        .dispatch_widget_output(
            11,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        )
        .expect("button should emit a host-defined increment message");
    bridge.reduce_message(increment);

    let surface_after = bridge.project_surface();
    assert_eq!(
        widget_ref::<TextWidget, _>(&surface_after, 10, "text").text,
        "Projects (1)"
    );
    assert_eq!(
        widget_ref::<TextInputWidget, _>(&surface_after, 12, "text input")
            .state
            .value,
        "Projects"
    );
}

#[test]
fn runtime_bridge_is_the_public_app_contract() {
    let mut bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
        },
    );

    let surface = project_app_once(&mut bridge);
    assert!(surface.find_widget(10).is_some());
}

#[test]
fn runtime_bridge_accepts_repaint_signal_for_host_background_work() {
    let called = Arc::new(AtomicBool::new(false));
    let mut bridge = RepaintSignalBridge::default();

    bridge.install_repaint_signal(Arc::new(CountingRepaintSignal {
        called: Arc::clone(&called),
    }));
    bridge.request_worker_repaint();

    assert!(called.load(Ordering::Acquire));
}

#[test]
fn runtime_bridge_exposes_host_owned_runtime_exit_artifact() {
    let mut bridge = RuntimeExitBridge;

    assert_eq!(
        bridge.on_runtime_exit(),
        Some(serde_json::json!({
            "status": "clean",
            "phase": "host-owned"
        }))
    );
}

#[test]
fn declarative_command_bridge_supports_command_update_flow() {
    let bridge = declarative_command_runtime_bridge(
        DemoState::default(),
        project_command_surface,
        |state: &mut DemoState, message| match message {
            CommandDemoMessage::Start => Command::batch([
                Command::message(CommandDemoMessage::Rename(String::from("Closure"))),
                Command::message(CommandDemoMessage::Increment),
                Command::request_repaint(),
            ]),
            CommandDemoMessage::Increment => {
                state.count += 1;
                Command::none()
            }
            CommandDemoMessage::Rename(name) => {
                state.name = name;
                Command::none()
            }
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    let outcome = runtime.dispatch_message(CommandDemoMessage::Start);

    assert_eq!(outcome.messages_dispatched, 3);
    assert!(outcome.repaint_requested);
    assert_eq!(runtime.bridge().state().count, 1);
    assert_eq!(runtime.bridge().state().name, "Closure");
}

#[test]
fn runtime_resource_slot_tracks_host_owned_background_results() {
    let mut preview = ResourceSlot::new("preview");

    preview.mark_loading();
    assert_eq!(preview.state(), ResourceLoadState::Loading);

    assert!(preview.apply(ResourceLoad::ready("preview", String::from("decoded"))));
    assert_eq!(preview.state(), ResourceLoadState::Ready);
    assert_eq!(preview.value().map(String::as_str), Some("decoded"));

    assert!(preview.apply(ResourceLoad::failed("preview", "invalid resource")));
    assert_eq!(preview.state(), ResourceLoadState::Failed);
    assert_eq!(preview.error(), Some("invalid resource"));
}

fn project_app_once(app: &mut impl App<DemoMessage>) -> Arc<UiSurface<DemoMessage>> {
    app.project_surface()
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

#[derive(Default)]
struct RepaintSignalBridge {
    signal: Option<Arc<dyn RepaintSignal>>,
}

impl RepaintSignalBridge {
    fn request_worker_repaint(&self) {
        if let Some(signal) = self.signal.as_ref() {
            signal.request_repaint();
        }
    }
}

impl RuntimeBridge<DemoMessage> for RepaintSignalBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        project_surface(&mut DemoState::default())
    }

    fn install_repaint_signal(&mut self, signal: Arc<dyn RepaintSignal>) {
        self.signal = Some(signal);
    }
}

struct CountingRepaintSignal {
    called: Arc<AtomicBool>,
}

impl RepaintSignal for CountingRepaintSignal {
    fn request_repaint(&self) {
        self.called.store(true, Ordering::Release);
    }
}

struct RuntimeExitBridge;

impl RuntimeBridge<DemoMessage> for RuntimeExitBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        project_surface(&mut DemoState::default())
    }

    fn on_runtime_exit(&mut self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "status": "clean",
            "phase": "host-owned"
        }))
    }
}

enum CommandDemoMessage {
    Start,
    Increment,
    Rename(String),
}

fn project_command_surface(state: &mut DemoState) -> Arc<UiSurface<CommandDemoMessage>> {
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
