//! Public API coverage for the generic `radiant::runtime` surface.

use radiant::prelude::IntoView;
use radiant::{
    gui::{
        focus::FocusSurface,
        input::{KeyCode, KeyPress},
        shortcuts::ShortcutResolution,
        types::{ImageRgba, Rgba8},
    },
    layout::{Point, Rect, Vector2, layout_tree, layout_tree_with_state},
    runtime::{
        Command, Element, Event, FocusTraversal, GpuSurfaceCapabilities, GpuSurfaceContent,
        GpuSurfaceLineStyle, GpuSurfaceOverlay, GpuSurfaceRuntimeOverlays, PaintPrimitive,
        Renderer, RepaintScope, RuntimeBridge, SurfaceChild, SurfaceNode, SurfacePaintPlan,
        SurfaceRuntime, UiSurface, View, WidgetMessageMapper, declarative_command_runtime_bridge,
        declarative_runtime_bridge,
    },
    theme::ThemeTokens,
    widgets::{
        ButtonWidget, CanvasMessage, DragHandleMessage, DragHandleWidget, GpuSurfaceParts,
        GpuSurfaceWidget, PointerButton, RetainedSurfaceDescriptor, TextEditCommand,
        TextInputWidget, TextWidget, Widget, WidgetInput, WidgetKey, WidgetProminence,
        WidgetSizing, WidgetState, WidgetStyle, WidgetTone, resolve_widget_visual_tokens,
    },
};
use std::sync::Arc;

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
#[path = "runtime_surface_public_api/commands.rs"]
mod commands;
#[cfg(test)]
#[path = "runtime_surface_public_api/focus_text.rs"]
mod focus_text;
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
