//! Stack animated normal widget overlays above a retained GPU surface.

use radiant::prelude::*;
use radiant::widgets::{PaintBounds, WidgetId};
use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

#[path = "gpu_surface_stack_overlay/gpu_content.rs"]
mod gpu_content;
#[path = "gpu_surface_stack_overlay/model.rs"]
mod model;
#[path = "gpu_surface_stack_overlay/selection_overlay.rs"]
mod selection_overlay;
#[path = "gpu_surface_stack_overlay/transient_overlay.rs"]
mod transient_overlay;
#[path = "gpu_surface_stack_overlay/view.rs"]
mod view;

#[cfg(test)]
#[path = "gpu_surface_stack_overlay/tests.rs"]
mod tests;

use model::{DemoMessage, DemoState};
use transient_overlay::paint_transient_blob;
use view::demo_view;

fn main() -> radiant::Result {
    radiant::app(DemoState::default())
        .title("Radiant GPU Surface Stack Overlay")
        .size(640, 344)
        .view(demo_view)
        .animated_transient_overlay_at(
            60,
            |state| state.running,
            |state, context, primitives| {
                paint_transient_blob(state, context.plan, context.animation_time, primitives);
            },
        )
        .handle_message(|state: &mut DemoState, message, context| match message {
            DemoMessage::ToggleSelection => {
                state.selected = !state.selected;
                context.request_repaint();
            }
            DemoMessage::ToggleAnimation => {
                state.running = !state.running;
                context.request_repaint();
            }
            DemoMessage::CommitResize { start, end } => {
                state.commit_selection(start, end);
                context.request_repaint();
            }
        })
        .run()
}
