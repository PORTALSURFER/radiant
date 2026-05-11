//! App-runtime API coverage for effects, startup commands, and repaint planning.

use radiant::{
    app,
    gui::{
        focus::FocusSurface,
        input::{KeyCode, KeyPress},
        repaint::RepaintSignal,
        shortcuts::ShortcutResolution,
        types::{ImageRgba, Point, Rect, Vector2},
    },
    layout::{Constraints, SizeModeCross, SizeModeMain, SlotParams},
    prelude::{GpuSurfaceContent, IntoView, gpu_surface, gpu_surface_input},
    runtime::{
        Command, Event, PaintPrimitive, RuntimeBridge, SurfaceChild, SurfaceNode, SurfaceRuntime,
        UiSurface,
    },
    theme::ThemeTokens,
    widgets::{
        ButtonWidget, PointerButton, TextInputWidget, TextWidget, WidgetInput, WidgetKey,
        WidgetSizing,
    },
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

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
        .to_string()
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
fn app_scroll_hook_observes_runtime_scroll_offsets() {
    let observed_scroll_y = Arc::new(Mutex::new(None));
    let observed_scroll_y_for_hook = Arc::clone(&observed_scroll_y);
    let bridge = app(DemoState::default())
        .view(|state: &mut DemoState| {
            UiSurface::new(SurfaceNode::scroll_area(
                20,
                SurfaceNode::column(
                    21,
                    0.0,
                    (0..12)
                        .map(|index| {
                            SurfaceChild::new(
                                intrinsic_slot(),
                                SurfaceNode::static_widget(TextWidget::new(
                                    100 + index,
                                    format!("Row {index} at {:.0}", state.last_scroll_y),
                                    WidgetSizing::fixed(Vector2::new(160.0, 28.0))
                                        .with_baseline(18.0),
                                )),
                            )
                        })
                        .collect(),
                ),
            ))
        })
        .on_scroll(move |state, update, context| {
            state.last_scroll_y = update.offset.y;
            *observed_scroll_y_for_hook
                .lock()
                .expect("scroll observer lock should be available") = Some(update.offset.y);
            context.request_paint_only();
        })
        .update_with(|_state, _message: DemoMessage, _context| {})
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 96.0));

    assert!(runtime.scroll_at(Point::new(20.0, 56.0), Vector2::new(0.0, 48.0)));

    assert_eq!(
        *observed_scroll_y
            .lock()
            .expect("scroll observer lock should be available"),
        Some(48.0)
    );
    assert_eq!(text_value(runtime.surface(), 100), "Row 0 at 48");
    assert!(runtime.take_repaint_requested());
}

#[test]
fn app_scroll_hook_observes_scrollbar_drag_offsets() {
    let observed_scroll_y = Arc::new(Mutex::new(None));
    let observed_scroll_y_for_hook = Arc::clone(&observed_scroll_y);
    let bridge = app(DemoState::default())
        .view(|state: &mut DemoState| {
            UiSurface::new(SurfaceNode::scroll_area(
                20,
                SurfaceNode::column(
                    21,
                    0.0,
                    (0..16)
                        .map(|index| {
                            SurfaceChild::new(
                                intrinsic_slot(),
                                SurfaceNode::static_widget(TextWidget::new(
                                    100 + index,
                                    format!("Row {index} at {:.0}", state.last_scroll_y),
                                    WidgetSizing::fixed(Vector2::new(160.0, 28.0))
                                        .with_baseline(18.0),
                                )),
                            )
                        })
                        .collect(),
                ),
            ))
        })
        .on_scroll(move |state, update, context| {
            state.last_scroll_y = update.offset.y;
            *observed_scroll_y_for_hook
                .lock()
                .expect("scroll observer lock should be available") = Some(update.offset.y);
            context.request_paint_only();
        })
        .update_with(|_state, _message: DemoMessage, _context| {})
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 96.0));
    let thumb = runtime
        .paint_plan(&ThemeTokens::default())
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) if fill.widget_id == 20 => Some(fill.rect),
            _ => None,
        })
        .expect("scroll area should paint a draggable thumb");

    runtime.dispatch_event(Event::PointerPress {
        position: thumb.center(),
        button: PointerButton::Primary,
    });
    runtime.dispatch_event(Event::PointerMove {
        position: Point::new(thumb.center().x, thumb.center().y + 36.0),
    });

    let observed = observed_scroll_y
        .lock()
        .expect("scroll observer lock should be available")
        .expect("scroll drag should notify host scroll hook");
    assert!(observed > 0.0);
    assert_eq!(
        text_value(runtime.surface(), 100),
        format!("Row 0 at {:.0}", observed)
    );
    assert!(runtime.take_repaint_requested());
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

#[test]
fn app_gpu_surface_builder_lowers_through_normal_view_path() {
    let atlas = Arc::new(ImageRgba::new(2, 1, vec![255; 8]).expect("valid atlas"));
    let view = radiant::prelude::row([gpu_surface::<DemoMessage>(
        41,
        7,
        GpuSurfaceContent::RgbaAtlas {
            source_rect: Rect::from_min_size(
                radiant::layout::Point::new(0.0, 0.0),
                Vector2::new(2.0, 1.0),
            ),
            atlas: Arc::clone(&atlas),
        },
    )
    .id(90)
    .size(240.0, 120.0)
    .width(240.0)
    .height(120.0)])
    .align_cross(radiant::layout::CrossAlign::Start);
    let surface = view.into_surface();
    let layout = radiant::layout::layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(
            radiant::layout::Point::new(0.0, 0.0),
            Vector2::new(320.0, 160.0),
        ),
    );

    let plan = surface.paint_plan(&layout, &ThemeTokens::default());

    let Some(PaintPrimitive::GpuSurface(gpu)) = plan.primitives.first() else {
        panic!("app GPU surface should emit a retained GPU paint primitive");
    };
    assert_eq!(gpu.widget_id, 90);
    assert_eq!(gpu.key, 41);
    assert_eq!(gpu.revision, 7);
    assert_eq!(
        gpu.rect,
        Rect::from_min_size(
            radiant::layout::Point::new(0.0, 0.0),
            Vector2::new(240.0, 120.0)
        )
    );
    let GpuSurfaceContent::RgbaAtlas { atlas: emitted, .. } = &gpu.content else {
        panic!("expected RGBA atlas content");
    };
    assert!(Arc::ptr_eq(&atlas, emitted));
}

#[test]
fn app_gpu_surface_input_helper_routes_through_normal_message_path() {
    let atlas = Arc::new(ImageRgba::new(2, 1, vec![255; 8]).expect("valid atlas"));
    let bridge = app(DemoState::default())
        .view(move |state: &mut DemoState| {
            radiant::prelude::column([
                radiant::prelude::text(format!("GPU inputs: {}", state.count)).id(91),
                gpu_surface_input(
                    41,
                    7,
                    GpuSurfaceContent::RgbaAtlas {
                        source_rect: Rect::from_min_size(
                            radiant::layout::Point::new(0.0, 0.0),
                            Vector2::new(2.0, 1.0),
                        ),
                        atlas: Arc::clone(&atlas),
                    },
                    DemoMessage::GpuInput,
                )
                .id(90)
                .size(240.0, 120.0),
            ])
        })
        .update_with(|state, message, _context| {
            if let DemoMessage::GpuInput(WidgetInput::PointerPress { .. }) = message {
                state.count += 1;
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(320.0, 160.0));

    let handled = runtime.dispatch_input(
        90,
        WidgetInput::PointerPress {
            position: radiant::layout::Point::new(24.0, 24.0),
            button: radiant::widgets::PointerButton::Primary,
        },
    );

    assert!(handled);
    assert_eq!(text_value(runtime.surface(), 91), "GPU inputs: 1");
}
