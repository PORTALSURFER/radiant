//! Large virtualized list using the public runtime surface API.

use radiant::{
    layout::{Constraints, SizeModeCross, SizeModeMain, SlotParams, Vector2, VirtualizationAxis},
    runtime::{SurfaceChild, SurfaceNode, UiSurface, declarative_runtime_bridge},
    widgets::WidgetSizing,
};
use std::sync::Arc;

const ROW_COUNT: usize = 10_000;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Message {
    Select(usize),
}

#[derive(Default)]
struct DemoState {
    selected: Option<usize>,
}

fn main() -> radiant::Result {
    let bridge = declarative_runtime_bridge(DemoState::default(), project_surface, reduce_message);
    radiant::window("Radiant Virtualized List")
        .size(420, 420)
        .min_size(320, 260)
        .run_bridge(bridge)
}

fn project_surface(state: &mut DemoState) -> Arc<UiSurface<Message>> {
    let rows = (0..ROW_COUNT)
        .map(|index| {
            let label = if Some(index) == state.selected {
                format!("Selected row {index:05}")
            } else {
                format!("Row {index:05}")
            };
            SurfaceChild::new(
                row_slot(),
                SurfaceNode::list_item_action(
                    1_000 + index as u64,
                    label,
                    WidgetSizing::fixed(Vector2::new(320.0, 28.0)),
                    Message::Select(index),
                ),
            )
        })
        .collect::<Vec<_>>();
    let list = SurfaceNode::virtual_scroll_area(
        20,
        SurfaceNode::column(21, 2.0, rows),
        VirtualizationAxis::Vertical,
        96.0,
    );
    let selected = state
        .selected
        .map(|index| format!("Selected: {index:05}"))
        .unwrap_or_else(|| String::from("Select a row"));

    Arc::new(UiSurface::new(SurfaceNode::column(
        1,
        10.0,
        vec![
            SurfaceChild::new(
                header_slot(),
                SurfaceNode::text(10, selected, WidgetSizing::fixed(Vector2::new(320.0, 28.0))),
            ),
            SurfaceChild::fill(list),
        ],
    )))
}

fn reduce_message(state: &mut DemoState, message: Message) {
    let Message::Select(index) = message;
    state.selected = Some(index);
}

fn header_slot() -> SlotParams {
    SlotParams {
        size_main: SizeModeMain::Intrinsic,
        size_cross: SizeModeCross::Fill,
        constraints: Constraints::unconstrained(),
        margin: Default::default(),
        align_cross_override: None,
        allow_fixed_compress: false,
    }
}

fn row_slot() -> SlotParams {
    SlotParams {
        size_main: SizeModeMain::Intrinsic,
        size_cross: SizeModeCross::Fill,
        constraints: Constraints::unconstrained(),
        margin: Default::default(),
        align_cross_override: None,
        allow_fixed_compress: true,
    }
}
