//! Public API coverage for the generic `radiant::runtime` surface.

use radiant::{
    layout::{ContainerKind, ContainerPolicy, Point, Rect, SlotParams, Vector2, layout_tree},
    runtime::{
        App, DEFAULT_NATIVE_WINDOW_TITLE, Element, FocusTraversal, NativeRunOptions,
        PaintPrimitive, Renderer, RuntimeBridge, SurfaceChild, SurfaceNode, SurfaceRuntime,
        UiSurface, View, WidgetMessageMapper, declarative_runtime_bridge,
    },
    theme::ThemeTokens,
    widgets::{
        ButtonMessage, ButtonWidget, TextInputMessage, TextInputWidget, TextWidget, WidgetInput,
        WidgetKey, WidgetSizing, WidgetSpec, WidgetState, WidgetStyle,
        resolve_widget_visual_tokens,
    },
};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
enum DemoMessage {
    Increment,
    Rename(String),
}

#[derive(Default)]
struct DemoState {
    count: usize,
    name: String,
}

#[test]
fn generic_runtime_surface_projects_layout_without_legacy_app_contracts() {
    let surface = project_surface(&mut DemoState::default());
    let output = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(420.0, 32.0)),
    );

    assert!(output.rects.contains_key(&10));
    assert!(output.rects.contains_key(&11));
    assert!(output.rects.contains_key(&12));
}

#[test]
fn native_run_options_default_uses_generic_radiant_title() {
    let options = NativeRunOptions::default();

    assert_eq!(options.title, DEFAULT_NATIVE_WINDOW_TITLE);
    assert_eq!(options.title, "Radiant");
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
            radiant::widgets::WidgetOutput::TextInput(TextInputMessage::Changed {
                value: String::from("Projects"),
            }),
        )
        .expect("text input should emit a host-defined rename message");
    bridge.reduce_message(rename);

    let increment = bridge
        .project_surface()
        .dispatch_widget_output(
            11,
            radiant::widgets::WidgetOutput::Button(ButtonMessage::Activate),
        )
        .expect("button should emit a host-defined increment message");
    bridge.reduce_message(increment);

    let surface_after = bridge.project_surface();
    let title = surface_after
        .find_widget(10)
        .expect("title widget should still be present")
        .widget();
    let field = surface_after
        .find_widget(12)
        .expect("text input widget should still be present")
        .widget();

    match title {
        WidgetSpec::Text(text) => assert_eq!(text.text, "Projects (1)"),
        other => panic!("expected text widget, got {other:?}"),
    }
    match field {
        WidgetSpec::TextInput(input) => assert_eq!(input.state.value, "Projects"),
        other => panic!("expected text input widget, got {other:?}"),
    }
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
fn view_and_element_aliases_match_runtime_surface_types() {
    let surface: Arc<View<DemoMessage>> = project_surface(&mut DemoState::default());
    let root: &Element<DemoMessage> = surface.root();

    assert_eq!(root.id(), 1);
    assert!(surface.find_widget(11).is_some());
}

#[test]
fn runtime_context_and_renderer_cover_paint_plan_boundary() {
    let theme = ThemeTokens::default();
    let bridge = declarative_runtime_bridge(
        DemoState {
            count: 3,
            name: String::from("Panels"),
        },
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
        },
    );
    let runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));
    let context = runtime.context();

    assert_eq!(context.viewport.width(), 420.0);
    assert!(context.surface.find_widget(11).is_some());
    assert!(context.layout.rects.contains_key(&11));

    let plan = runtime.paint_plan(&theme);
    let mut renderer = CountingRenderer::default();
    renderer
        .render(&plan)
        .expect("counting renderer cannot fail");
    assert_eq!(renderer.rendered_primitives, plan.primitives.len());
}

#[test]
fn surface_runtime_manages_focus_and_routes_keyboard_to_focused_widget() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    assert_eq!(runtime.focused_widget(), None);
    assert_eq!(runtime.traverse_focus(FocusTraversal::Forward), Some(11));
    assert_eq!(runtime.focused_widget(), Some(11));
    assert_eq!(runtime.traverse_focus(FocusTraversal::Forward), Some(12));
    assert_eq!(runtime.traverse_focus(FocusTraversal::Backward), Some(11));
    assert_eq!(
        runtime.dispatch_focused_input(WidgetInput::KeyPress(WidgetKey::Enter)),
        Some(11)
    );

    let title = runtime
        .surface()
        .find_widget(10)
        .expect("title widget should still be present")
        .widget();
    match title {
        WidgetSpec::Text(text) => assert_eq!(text.text, "Untitled (1)"),
        other => panic!("expected text widget, got {other:?}"),
    }

    assert!(runtime.focus_widget(12));
    assert_eq!(
        runtime.dispatch_focused_input(WidgetInput::Character('Q')),
        Some(12)
    );
    runtime.clear_focus();
    assert_eq!(runtime.focused_widget(), None);
    assert_eq!(
        runtime.dispatch_focused_input(WidgetInput::Character('X')),
        None
    );

    let field = runtime
        .surface()
        .find_widget(12)
        .expect("text input widget should still be present")
        .widget();
    match field {
        WidgetSpec::TextInput(input) => assert_eq!(input.state.value, "Q"),
        other => panic!("expected text input widget, got {other:?}"),
    }
}

#[test]
fn surface_runtime_routes_widget_input_and_reprojects_surface() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));
    let input_bounds = runtime
        .layout()
        .rects
        .get(&12)
        .copied()
        .expect("text input should have layout bounds");
    let input_point = Point::new(
        input_bounds.min.x + input_bounds.width() * 0.5,
        input_bounds.min.y + input_bounds.height() * 0.5,
    );

    assert_eq!(runtime.widget_at(Point::new(150.0, 10.0)), Some(11));
    assert!(runtime.dispatch_input(12, WidgetInput::FocusChanged(true)));
    assert!(runtime.dispatch_input(12, WidgetInput::Character('F')));
    assert!(runtime.dispatch_input(11, WidgetInput::FocusChanged(true)));
    assert_eq!(
        runtime.dispatch_input_at(input_point, WidgetInput::FocusChanged(true)),
        Some(12)
    );
    assert!(runtime.dispatch_input(11, WidgetInput::KeyPress(WidgetKey::Enter)));

    let title = runtime
        .surface()
        .find_widget(10)
        .expect("title widget should still be present")
        .widget();
    let field = runtime
        .surface()
        .find_widget(12)
        .expect("text input widget should still be present")
        .widget();

    match title {
        WidgetSpec::Text(text) => assert_eq!(text.text, "F (1)"),
        other => panic!("expected text widget, got {other:?}"),
    }
    match field {
        WidgetSpec::TextInput(input) => assert_eq!(input.state.value, "F"),
        other => panic!("expected text input widget, got {other:?}"),
    }
}

#[derive(Default)]
struct CountingRenderer {
    rendered_primitives: usize,
}

impl Renderer for CountingRenderer {
    type Error = std::convert::Infallible;

    fn render(&mut self, plan: &radiant::runtime::SurfacePaintPlan) -> Result<(), Self::Error> {
        self.rendered_primitives += plan.primitives.len();
        Ok(())
    }
}

fn project_app_once(app: &mut impl App<DemoMessage>) -> Arc<UiSurface<DemoMessage>> {
    app.project_surface()
}

#[test]
fn generic_public_surface_resolves_theme_without_legacy_shell_contracts() {
    let theme = ThemeTokens::default();
    let visuals = resolve_widget_visual_tokens(
        &theme,
        WidgetStyle::default(),
        WidgetState {
            focused: true,
            ..WidgetState::default()
        },
    );

    assert_eq!(visuals.border, theme.border_emphasis);
}

#[test]
fn generic_surface_projects_deterministic_paint_without_legacy_shell_contracts() {
    let theme = ThemeTokens::default();
    let bridge = declarative_runtime_bridge(
        DemoState {
            count: 2,
            name: String::from("Crates"),
        },
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
        },
    );
    let runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    let direct_plan = runtime.surface().paint_plan(runtime.layout(), &theme);
    let runtime_plan = runtime.paint_plan(&theme);

    assert_eq!(runtime_plan, direct_plan);
    assert_eq!(runtime_plan.clear_color, theme.clear_color);
    assert_eq!(runtime_plan.primitives.len(), 7);

    let texts: Vec<_> = runtime_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::Text(text) => Some((text.widget_id, text.text.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(
        texts,
        vec![(10, "Crates (2)"), (11, "Increment"), (12, "Crates")]
    );

    let fills: Vec<_> = runtime_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) => Some((fill.widget_id, fill.color)),
            _ => None,
        })
        .collect();
    assert_eq!(
        fills,
        vec![(11, theme.surface_raised), (12, theme.surface_raised)]
    );
}

fn project_surface(state: &mut DemoState) -> Arc<UiSurface<DemoMessage>> {
    let title = WidgetSpec::Text(TextWidget::new(
        10,
        format!("{} ({})", display_name(state), state.count),
        WidgetSizing::fixed(Vector2::new(140.0, 20.0)).with_baseline(14.0),
    ));
    let button = WidgetSpec::Button(ButtonWidget::new(
        11,
        "Increment",
        WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
    ));
    let input = WidgetSpec::TextInput(TextInputWidget::new(
        12,
        state.name.clone(),
        WidgetSizing::new(Vector2::new(120.0, 28.0), Vector2::new(180.0, 28.0)),
    ));

    Arc::new(UiSurface::new(SurfaceNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Row,
            spacing: 8.0,
            ..ContainerPolicy::default()
        },
        vec![
            SurfaceChild::new(
                SlotParams::fill(),
                SurfaceNode::widget(title, WidgetMessageMapper::None),
            ),
            SurfaceChild::new(
                SlotParams::fill(),
                SurfaceNode::widget(
                    button,
                    WidgetMessageMapper::button(|_| DemoMessage::Increment),
                ),
            ),
            SurfaceChild::new(
                SlotParams::fill(),
                SurfaceNode::widget(
                    input,
                    WidgetMessageMapper::text_input(|message| match message {
                        TextInputMessage::Changed { value }
                        | TextInputMessage::Submitted { value } => DemoMessage::Rename(value),
                    }),
                ),
            ),
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
