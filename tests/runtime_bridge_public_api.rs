//! Public API coverage for generic runtime bridge contracts.

use radiant::{
    layout::Vector2,
    runtime::{
        App, RuntimeBridge, SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper,
        declarative_owned_runtime_bridge, declarative_runtime_bridge,
    },
    widgets::{
        ButtonMessage, ButtonWidget, TextInputMessage, TextInputWidget, TextWidget, Widget,
        WidgetSizing,
    },
};
use std::sync::Arc;

#[path = "runtime_bridge_public_api/command_flow.rs"]
mod command_flow;
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
