//! Public API coverage for the generic `radiant::runtime` surface.

use radiant::prelude::IntoView;
use radiant::{
    gui::{
        focus::FocusSurface,
        input::{KeyCode, KeyPress},
        shortcuts::ShortcutResolution,
        types::{ImageRgba, Rgba8},
    },
    layout::{Point, Rect, Vector2, VirtualizationAxis, layout_tree},
    runtime::{
        Command, Element, Event, FocusTraversal, GpuHoverCursor, GpuSurfaceCapabilities,
        GpuSurfaceContent, GpuSurfaceOverlay, PaintPrimitive, Renderer, RuntimeBridge,
        SurfaceChild, SurfaceNode, SurfaceRuntime, UiSurface, View, WidgetMessageMapper,
        declarative_command_runtime_bridge, declarative_runtime_bridge,
    },
    theme::ThemeTokens,
    widgets::{
        ButtonWidget, CanvasMessage, DragHandleMessage, DragHandleWidget, GpuSurfaceWidget,
        PointerButton, RetainedSurfaceDescriptor, TextEditCommand, TextInputWidget, TextWidget,
        Widget, WidgetCommon, WidgetInput, WidgetKey, WidgetProminence, WidgetSizing, WidgetState,
        WidgetStyle, WidgetTone, resolve_widget_visual_tokens,
    },
};
use std::sync::Arc;
use std::time::Duration;

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

#[cfg(test)]
#[path = "runtime_surface_public_api/interaction.rs"]
mod interaction;
#[cfg(test)]
#[path = "runtime_surface_public_api/paint_projection.rs"]
mod paint_projection;

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
    std::thread::sleep(Duration::from_millis(20));
    let drained = runtime.drain_runtime_messages();
    assert_eq!(drained.messages_dispatched, 1);
    assert_eq!(runtime.bridge().count, 1);

    let performed = runtime.execute_command(Command::perform(
        "increment",
        || DemoMessage::Increment,
        |message| message,
    ));
    assert!(performed.repaint_requested);
    std::thread::sleep(Duration::from_millis(20));
    let drained = runtime.drain_runtime_messages();
    assert_eq!(drained.messages_dispatched, 1);
    assert_eq!(runtime.bridge().count, 2);

    let exit = runtime.execute_command(Command::exit());
    assert!(exit.exit_requested);
    assert!(runtime.take_exit_requested());
}

#[test]
fn surface_runtime_scrolls_virtual_list_with_cached_layout_and_bounded_paint_plan() {
    let bridge = declarative_runtime_bridge(
        (),
        |_state: &mut ()| {
            let rows = (0..10_000_u64)
                .map(|index| {
                    SurfaceChild::fill(SurfaceNode::static_widget(TextWidget::new(
                        index + 10,
                        format!("Row {index:05}"),
                        WidgetSizing::fixed(Vector2::new(160.0, 28.0)).with_baseline(18.0),
                    )))
                })
                .collect::<Vec<_>>();
            Arc::new(UiSurface::new(SurfaceNode::virtual_scroll_area(
                1,
                SurfaceNode::column(2, 4.0, rows),
                VirtualizationAxis::Vertical,
                96.0,
            )))
        },
        |_state: &mut (), _message: DemoMessage| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 120.0));

    assert!(runtime.scroll_at(Point::new(24.0, 24.0), Vector2::new(0.0, 3_000.0)));

    let layout = runtime.layout();
    let window = layout
        .virtual_windows
        .get(&1)
        .expect("virtual scroll window should be resolved");
    assert!(window.first_index > 0);
    assert!(window.last_index_exclusive - window.first_index < 128);
    assert!(
        layout.stats.measured_nodes < 64,
        "scroll relayout should reuse virtual metrics instead of measuring the full list"
    );

    let paint = runtime.paint_plan(&ThemeTokens::default());
    assert!(
        paint.primitives.len() < 160,
        "virtual scroll paint should stay bounded to the materialized window"
    );
}

#[test]
fn surface_runtime_skips_non_wheel_widgets_before_virtual_scroll_fallback() {
    let bridge = declarative_runtime_bridge(
        (),
        |_state: &mut ()| {
            let rows = (0..10_000_u64)
                .map(|index| {
                    SurfaceChild::fill(SurfaceNode::custom_widget(
                        PanicOnWheelWidget::new(index + 10),
                        WidgetMessageMapper::typed(|_: DemoMessage| DemoMessage::Increment),
                    ))
                })
                .collect::<Vec<_>>();
            Arc::new(UiSurface::new(SurfaceNode::virtual_scroll_area(
                1,
                SurfaceNode::column(2, 4.0, rows),
                VirtualizationAxis::Vertical,
                96.0,
            )))
        },
        |_state: &mut (), _message: DemoMessage| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 120.0));

    assert!(runtime.wheel_or_scroll_at(Point::new(24.0, 24.0), Vector2::new(0.0, 3_000.0)));

    let window = runtime
        .layout()
        .virtual_windows
        .get(&1)
        .expect("virtual scroll window should be resolved");
    assert!(window.first_index > 0);
}

#[test]
fn surface_runtime_skips_stable_pointer_motion_for_opted_out_widgets() {
    let bridge = pointer_motion_bridge(false);
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(140.0, 60.0));

    assert_eq!(
        runtime.dispatch_event(Event::PointerMove {
            position: Point::new(16.0, 16.0),
        }),
        Some(10)
    );
    assert_eq!(
        runtime.dispatch_event(Event::PointerMove {
            position: Point::new(20.0, 20.0),
        }),
        Some(10)
    );

    let probe = widget_ref::<PointerMotionProbeWidget, _>(runtime.surface(), 10, "motion probe");
    assert_eq!(probe.moves, 1);
    assert!(probe.common.state.hovered);
}

#[test]
fn surface_runtime_preserves_stable_pointer_motion_for_continuous_widgets() {
    let bridge = pointer_motion_bridge(true);
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(140.0, 60.0));

    let _ = runtime.dispatch_event(Event::PointerMove {
        position: Point::new(16.0, 16.0),
    });
    let _ = runtime.dispatch_event(Event::PointerMove {
        position: Point::new(20.0, 20.0),
    });

    let probe = widget_ref::<PointerMotionProbeWidget, _>(runtime.surface(), 10, "motion probe");
    assert_eq!(probe.moves, 2);
}

#[test]
fn surface_runtime_keeps_captured_pointer_motion_for_opted_out_widgets() {
    let bridge = pointer_motion_bridge(false);
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(140.0, 60.0));

    let _ = runtime.dispatch_event(Event::PointerPress {
        position: Point::new(16.0, 16.0),
        button: PointerButton::Primary,
    });
    let _ = runtime.dispatch_event(Event::PointerMove {
        position: Point::new(18.0, 18.0),
    });
    let _ = runtime.dispatch_event(Event::PointerMove {
        position: Point::new(20.0, 20.0),
    });

    let probe = widget_ref::<PointerMotionProbeWidget, _>(runtime.surface(), 10, "motion probe");
    assert_eq!(probe.moves, 2);
    assert!(probe.common.state.pressed);
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

fn pointer_motion_bridge(continuous_pointer_move: bool) -> impl RuntimeBridge<DemoMessage> {
    declarative_runtime_bridge(
        continuous_pointer_move,
        |continuous_pointer_move: &mut bool| {
            Arc::new(UiSurface::new(SurfaceNode::custom_widget(
                PointerMotionProbeWidget::new(10, *continuous_pointer_move),
                WidgetMessageMapper::none(),
            )))
        },
        |_continuous_pointer_move: &mut bool, _message| {},
    )
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

#[derive(Clone, Debug)]
struct PanicOnWheelWidget {
    common: WidgetCommon,
}

#[derive(Clone, Debug)]
struct PointerMotionProbeWidget {
    common: WidgetCommon,
    continuous_pointer_move: bool,
    moves: usize,
}

impl PointerMotionProbeWidget {
    fn new(id: u64, continuous_pointer_move: bool) -> Self {
        let mut common = WidgetCommon::new(
            id,
            WidgetSizing::fixed(Vector2::new(120.0, 40.0)).with_baseline(24.0),
        );
        common.focus = radiant::widgets::FocusBehavior::Pointer;
        Self {
            common,
            continuous_pointer_move,
            moves: 0,
        }
    }
}

impl Widget for PointerMotionProbeWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn accepts_pointer_move(&self) -> bool {
        self.continuous_pointer_move
    }

    fn handle_input(
        &mut self,
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<radiant::widgets::WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position } => {
                self.moves += 1;
                self.common.state.hovered = bounds.contains(position);
            }
            WidgetInput::PointerPress { position, .. } => {
                self.common.state.hovered = bounds.contains(position);
                self.common.state.pressed = bounds.contains(position);
            }
            WidgetInput::PointerRelease { position, .. } => {
                self.common.state.hovered = bounds.contains(position);
                self.common.state.pressed = false;
            }
            _ => {}
        }
        None
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &radiant::layout::LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}

impl PanicOnWheelWidget {
    fn new(id: u64) -> Self {
        let mut common = WidgetCommon::new(
            id,
            WidgetSizing::fixed(Vector2::new(160.0, 28.0)).with_baseline(18.0),
        );
        common.focus = radiant::widgets::FocusBehavior::Pointer;
        Self { common }
    }
}

impl Widget for PanicOnWheelWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(
        &mut self,
        _bounds: Rect,
        input: WidgetInput,
    ) -> Option<radiant::widgets::WidgetOutput> {
        if matches!(input, WidgetInput::Wheel { .. }) {
            panic!("wheel input should skip widgets that do not opt into wheel routing");
        }
        None
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &radiant::layout::LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
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
