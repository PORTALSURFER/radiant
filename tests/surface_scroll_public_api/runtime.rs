use super::{DemoMessage, intrinsic_slot};
use radiant::{
    layout::{Point, Vector2},
    runtime::{
        Command, RuntimeBridge, ScrollFixedRowIntoViewParts, ScrollIntoViewParts, ScrollUpdate,
        SurfaceChild, SurfaceNode, SurfaceRuntime, UiSurface, declarative_runtime_bridge,
    },
    widgets::WidgetSizing,
};
use std::sync::Arc;

#[path = "runtime/scrollbar_affordance.rs"]
mod scrollbar_affordance;

struct ScrollObserverBridge {
    surface: Arc<UiSurface<DemoMessage>>,
    updates: usize,
    last_update: Option<ScrollUpdate>,
}

impl RuntimeBridge<DemoMessage> for ScrollObserverBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        Arc::clone(&self.surface)
    }

    fn scroll_updated(&mut self, update: ScrollUpdate) -> Option<Command<DemoMessage>> {
        self.updates += 1;
        self.last_update = Some(update);
        None
    }
}

#[path = "runtime/commands.rs"]
mod commands;
#[path = "runtime/pointer_routing.rs"]
mod pointer_routing;
