//! App-runtime API coverage for effects, startup commands, and repaint planning.

use radiant::{
    app,
    gui::{
        focus::FocusSurface,
        input::{KeyCode, KeyPress},
        repaint::RepaintSignal,
        shortcuts::{
            ShortcutCatalog, ShortcutGesture, ShortcutLayer, ShortcutResolution, ShortcutStack,
        },
        types::{Point, Rect, Rgba8, Vector2},
    },
    layout::{Constraints, SizeModeCross, SizeModeMain, SlotParams},
    prelude::ViewProjection,
    runtime::{
        Command, PaintFillRect, PaintPrimitive, RuntimeBridge, RuntimeHostCapabilities,
        RuntimeQueueHost, SurfaceNode, SurfaceRuntime, UiSurface,
    },
    theme::ThemeTokens,
    widgets::{ButtonWidget, TextWidget, WidgetInput, WidgetKey, WidgetSizing},
};
use std::sync::Arc;

#[path = "app_runtime_api/lifecycle.rs"]
mod lifecycle;
#[path = "app_runtime_api/paint_overlay.rs"]
mod paint_overlay;
#[path = "app_runtime_api/render_canvas.rs"]
mod render_canvas;
#[path = "app_runtime_api/runtime_diagnostics.rs"]
mod runtime_diagnostics;
#[path = "app_runtime_api/runtime_work.rs"]
mod runtime_work;
#[path = "app_runtime_api/scroll_hooks.rs"]
mod scroll_hooks;
#[path = "app_runtime_api/shortcuts.rs"]
mod shortcuts;
#[path = "app_runtime_api/tasks_platform.rs"]
mod tasks_platform;

#[derive(Clone, Debug, PartialEq)]
enum DemoMessage {
    Increment,
    Noop,
    CanvasInput(WidgetInput),
    VirtualListWindowChanged(radiant::prelude::VirtualListWindowChange),
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

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, DemoMessage> {
        RuntimeHostCapabilities::new().with_queues()
    }
}

impl RuntimeQueueHost<DemoMessage> for DrainIntoBridge {
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
