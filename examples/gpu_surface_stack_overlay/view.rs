use super::*;
use crate::gpu_content::demo_gpu_content;
use crate::model::{DemoMessage, DemoState};
use crate::selection_overlay::SelectionOverlay;

pub(super) const SURFACE_WIDTH: f32 = 560.0;
pub(super) const SURFACE_HEIGHT: f32 = 220.0;

pub(super) fn demo_view(state: &DemoState) -> View<DemoMessage> {
    column([
        row([
            button(if state.running { "Pause" } else { "Run" })
                .message(DemoMessage::ToggleAnimation)
                .id(1)
                .width(88.0)
                .height(32.0),
            text(format!(
                "selection {:.0}% - {:.0}%",
                state.selection_start * 100.0,
                state.selection_end * 100.0
            ))
            .id(2)
            .fill_width()
            .height(32.0),
        ])
        .spacing(12.0)
        .fill_width(),
        stack([
            gpu_surface(42, 1, demo_gpu_content())
                .id(10)
                .size(SURFACE_WIDTH, SURFACE_HEIGHT),
            custom_widget_mapped(SelectionOverlay::new(state), |message| message)
                .id(11)
                .size(SURFACE_WIDTH, SURFACE_HEIGHT),
        ])
        .id(12)
        .size(SURFACE_WIDTH, SURFACE_HEIGHT),
    ])
    .id(100)
    .padding(24.0)
    .spacing(16.0)
}
