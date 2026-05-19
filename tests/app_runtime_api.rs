//! App-runtime API coverage for effects, startup commands, and repaint planning.

use radiant::{
    app,
    gui::{
        focus::FocusSurface,
        input::{KeyCode, KeyPress},
        repaint::RepaintSignal,
        shortcuts::{ShortcutGesture, ShortcutLayer, ShortcutResolution},
        types::{Point, Rect, Rgba8, Vector2},
    },
    layout::{Constraints, SizeModeCross, SizeModeMain, SlotParams},
    runtime::{
        Command, PaintFillRect, PaintPrimitive, RuntimeBridge, SurfaceNode, SurfaceRuntime,
        UiSurface,
    },
    theme::ThemeTokens,
    widgets::{ButtonWidget, TextWidget, WidgetInput, WidgetKey, WidgetSizing},
};
use std::sync::Arc;

#[path = "app_runtime_api/gpu_surface.rs"]
mod gpu_surface;
#[path = "app_runtime_api/lifecycle.rs"]
mod lifecycle;
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

#[derive(Default)]
struct DrainIntoBridge {
    commands: Vec<Command<DemoMessage>>,
    messages: Vec<DemoMessage>,
    drained_commands_into: bool,
    drained_messages_into: bool,
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

#[test]
fn background_message_drains_are_budgeted_to_preserve_ui_responsiveness() {
    let bridge = DrainIntoBridge {
        messages: vec![DemoMessage::Increment; 65],
        ..DrainIntoBridge::default()
    };
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));

    let first = runtime.drain_runtime_messages();

    assert_eq!(first.messages_dispatched, 64);
    assert!(first.runtime_work_remaining);
    assert!(first.repaint_requested);
    assert!(runtime.take_repaint_requested());

    let second = runtime.drain_runtime_messages();

    assert_eq!(second.messages_dispatched, 1);
    assert!(!second.runtime_work_remaining);
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
fn shortcut_layer_public_api_handles_modal_layers_and_dynamic_fallbacks() {
    let modal = ShortcutLayer::modal().bind(KeyPress::new(KeyCode::Escape), DemoMessage::Increment);
    assert_eq!(
        modal.resolve(KeyPress::new(KeyCode::Escape)),
        ShortcutResolution::action(DemoMessage::Increment)
    );
    assert_eq!(
        modal.resolve(KeyPress::new(KeyCode::N)),
        ShortcutResolution::handled()
    );

    let global = ShortcutLayer::new().bind(
        ShortcutGesture::with_command(KeyCode::A),
        DemoMessage::Increment,
    );
    assert_eq!(
        global.resolve_or_else(KeyPress::new(KeyCode::ArrowDown), || {
            ShortcutResolution::action(DemoMessage::Increment)
        }),
        ShortcutResolution::action(DemoMessage::Increment)
    );
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
fn app_transient_overlay_painter_reads_state_and_cached_plan() {
    let bridge = app(DemoState::default())
        .view(|state: &mut DemoState| {
            radiant::prelude::text(format!("Count {}", state.count)).id(10)
        })
        .transient_overlay(|state, context, primitives| {
            assert_eq!(context.viewport, Vector2::new(180.0, 40.0));
            let Some(text) = context
                .plan
                .primitives
                .iter()
                .find_map(|primitive| match primitive {
                    PaintPrimitive::Text(text) => Some(text),
                    _ => None,
                })
            else {
                return;
            };
            primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: text.widget_id,
                rect: Rect::from_min_size(Point::new(4.0, 4.0), Vector2::new(8.0, 8.0)),
                color: Rgba8 {
                    r: state.count as u8,
                    g: 128,
                    b: 255,
                    a: 255,
                },
            }));
        })
        .update_with(|state, message, context| {
            if matches!(message, DemoMessage::Increment) {
                state.count += 1;
                context.request_paint_only();
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));
    let plan = runtime.paint_plan(&ThemeTokens::default());
    let _ = runtime.dispatch_message(DemoMessage::Increment);
    let mut overlay = Vec::new();

    runtime.bridge_mut().paint_transient_overlay(
        radiant::runtime::TransientOverlayContext::new(
            &plan,
            Vector2::new(180.0, 40.0),
            std::time::Duration::ZERO,
        ),
        &mut overlay,
    );

    let [PaintPrimitive::FillRect(fill)] = overlay.as_slice() else {
        panic!("expected one transient fill rect");
    };
    assert_eq!(fill.color.r, 1);
}

#[test]
fn latest_task_tracks_current_ticket_and_tags_spawned_completion() {
    let mut latest = radiant::prelude::LatestTask::new();
    let first = latest.begin();
    let second = latest.begin();

    assert!(!latest.is_active(first));
    assert!(latest.is_active(second));
    assert!(!latest.finish(first));
    assert!(latest.finish(second));
    assert_eq!(latest.active(), None);

    let mut latest = radiant::prelude::LatestTask::new();
    let mut context = radiant::prelude::UpdateContext::default();
    context.spawn_latest(
        &mut latest,
        "latest-task-test",
        || 7_u32,
        |completion| {
            assert_eq!(completion.task_id(), 1);
            DemoMessage::Increment
        },
    );

    assert_eq!(latest.active().map(|ticket| ticket.id()), Some(1));
}

#[test]
fn update_context_exposes_platform_service_helpers() {
    let mut context = radiant::prelude::UpdateContext::default();
    context.pick_folder(
        radiant::runtime::FileDialogRequest::new().title("Choose library"),
        |_| DemoMessage::Increment,
    );
    context.pick_file(
        radiant::runtime::FileDialogRequest::new().filter("Wave", vec![String::from("wav")]),
        |_| DemoMessage::Increment,
    );
    context.save_file(
        radiant::runtime::FileDialogRequest::new().filename("export.wav"),
        |_| DemoMessage::Increment,
    );
    context.open_path(std::path::PathBuf::from(r"C:\samples"), |_| {
        DemoMessage::Increment
    });
    context.open_url("https://example.invalid", |_| DemoMessage::Increment);
    context.confirm(
        radiant::runtime::ConfirmDialogRequest::new("Delete sample", "Delete selected sample?"),
        |_| DemoMessage::Increment,
    );
}

#[test]
fn confirm_dialog_supports_named_parts_construction() {
    let request =
        radiant::prelude::ConfirmDialogRequest::from_parts(radiant::prelude::ConfirmDialogParts {
            title: "Overwrite file".to_owned(),
            message: "Replace the existing export?".to_owned(),
            level: radiant::prelude::ConfirmationLevel::Warning,
            buttons: radiant::prelude::ConfirmationButtons::YesNo,
        });

    assert_eq!(request.title, "Overwrite file");
    assert_eq!(request.message, "Replace the existing export?");
    assert_eq!(request.level, radiant::prelude::ConfirmationLevel::Warning);
    assert_eq!(
        request.buttons,
        radiant::prelude::ConfirmationButtons::YesNo
    );
}

#[test]
fn update_context_can_spawn_cancellable_work() {
    let token = radiant::prelude::CancellationToken::new();
    let worker_token = token.clone();
    token.cancel();

    let mut context = radiant::prelude::UpdateContext::default();
    context.spawn_cancellable(
        "cancel-test",
        worker_token,
        |token| token.is_cancelled(),
        |cancelled| {
            assert!(cancelled);
            DemoMessage::Increment
        },
    );
}
