//! Public API coverage for generic runtime bridge contracts.

use radiant::{
    layout::{Constraints, SizeModeCross, SizeModeMain, SlotParams, Vector2},
    runtime::{
        App, DeclarativeOwnedRuntimeBridge, DeclarativeOwnedRuntimeBridgeParts,
        DeclarativeOwnedSurfaceRuntime, DeclarativeRuntimeBridge, DeclarativeRuntimeBridgeParts,
        DeclarativeSurfaceRuntime, RuntimeBridge, SurfaceChild, SurfaceNode, SurfaceRuntime,
        UiSurface, WidgetMessageMapper, declarative_owned_runtime_bridge,
        declarative_runtime_bridge,
    },
    widgets::{
        ButtonMessage, ButtonWidget, TextInputMessage, TextInputWidget, TextWidget, Widget,
        WidgetSizing,
    },
};
use std::sync::Arc;

#[path = "runtime_bridge_public_api/command_flow.rs"]
mod command_flow;
#[path = "runtime_bridge_public_api/diagnostics.rs"]
mod diagnostics;
#[path = "runtime_bridge_public_api/lifecycle.rs"]
mod lifecycle;
#[path = "runtime_bridge_public_api/resources.rs"]
mod resources;
#[path = "runtime_bridge_public_api/state_projection.rs"]
mod state_projection;

#[derive(Clone, Debug, PartialEq)]
enum DemoMessage {
    Increment,
    Rename(String),
}

#[derive(Default)]
struct DemoState {
    count: usize,
    name: String,
    show_list: bool,
    scroll_reports: Vec<f32>,
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

fn project_owned_surface(state: &mut DemoState) -> UiSurface<DemoMessage> {
    Arc::unwrap_or_clone(project_surface(state))
}

fn display_name(state: &DemoState) -> &str {
    if state.name.is_empty() {
        "Untitled"
    } else {
        &state.name
    }
}

type SharedDemoRuntime = DeclarativeSurfaceRuntime<
    DemoState,
    DemoMessage,
    fn(&mut DemoState) -> Arc<UiSurface<DemoMessage>>,
    fn(&mut DemoState, DemoMessage),
>;

type OwnedDemoRuntime = DeclarativeOwnedSurfaceRuntime<
    DemoState,
    DemoMessage,
    fn(&mut DemoState) -> UiSurface<DemoMessage>,
    fn(&mut DemoState, DemoMessage),
>;

#[test]
fn declarative_runtime_bridges_support_named_parts_construction() {
    let mut bridge = DeclarativeRuntimeBridge::from_parts(DeclarativeRuntimeBridgeParts {
        state: DemoState::default(),
        project: project_surface,
        reduce: |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
        },
    });
    bridge.reduce_message(DemoMessage::Increment);
    assert_eq!(bridge.state().count, 1);
    assert!(bridge.project_surface().find_widget(11).is_some());

    let mut owned_bridge =
        DeclarativeOwnedRuntimeBridge::from_parts(DeclarativeOwnedRuntimeBridgeParts {
            state: DemoState::default(),
            project: project_owned_surface,
            reduce: |state: &mut DemoState, message| match message {
                DemoMessage::Increment => state.count += 1,
                DemoMessage::Rename(name) => state.name = name,
            },
        });
    owned_bridge.reduce_message(DemoMessage::Rename(String::from("Owned")));
    assert_eq!(owned_bridge.state().name, "Owned");
    assert!(owned_bridge.pull_surface().find_widget(12).is_some());
}

#[test]
fn surface_runtime_can_build_declarative_bridges_directly() {
    let mut runtime = SurfaceRuntime::new_declarative(
        DemoState::default(),
        Vector2::new(320.0, 120.0),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
        },
    );
    runtime.dispatch_message(DemoMessage::Increment);
    assert_eq!(runtime.bridge().state().count, 1);
    assert!(
        runtime
            .frame_with_default_theme()
            .paint_plan
            .contains_text("Untitled (1)")
    );

    let mut owned_runtime = SurfaceRuntime::new_declarative_owned(
        DemoState::default(),
        Vector2::new(320.0, 120.0),
        project_owned_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
        },
    );
    owned_runtime.dispatch_message(DemoMessage::Rename(String::from("Owned")));
    assert_eq!(owned_runtime.bridge().state().name, "Owned");
    assert!(
        owned_runtime
            .frame_with_default_theme()
            .paint_plan
            .contains_text("Owned (0)")
    );
}

#[test]
fn declarative_surface_runtime_aliases_name_common_host_shapes() {
    let mut shared: SharedDemoRuntime = SurfaceRuntime::new_declarative(
        DemoState::default(),
        Vector2::new(320.0, 120.0),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
        },
    );
    shared.dispatch_message(DemoMessage::Increment);
    assert_eq!(shared.bridge().state().count, 1);

    let mut owned: OwnedDemoRuntime = SurfaceRuntime::new_declarative_owned(
        DemoState::default(),
        Vector2::new(320.0, 120.0),
        project_owned_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
        },
    );
    owned.dispatch_message(DemoMessage::Rename(String::from("Alias")));
    assert_eq!(owned.bridge().state().name, "Alias");
}
