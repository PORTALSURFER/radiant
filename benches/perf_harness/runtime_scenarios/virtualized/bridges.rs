//! Synthetic bridges and surface trees for virtualized runtime benchmarks.

use radiant::{
    layout::{SizeModeCross, SizeModeMain, SlotParams, Vector2, VirtualizationAxis},
    runtime::{RuntimeBridge, SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper},
    widgets::{ButtonWidget, WidgetSizing},
};
use std::sync::Arc;

pub(super) struct VirtualWheelBridge;

impl RuntimeBridge<()> for VirtualWheelBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        Arc::new(UiSurface::new(SurfaceNode::virtual_scroll_area(
            1,
            SurfaceNode::column(2, 4.0, virtual_button_rows(10_000)),
            VirtualizationAxis::Vertical,
            96.0,
        )))
    }
}

pub(super) struct NestedScrollBridge;

impl RuntimeBridge<()> for NestedScrollBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        Arc::new(UiSurface::new(SurfaceNode::virtual_scroll_area(
            1,
            SurfaceNode::column(2, 2.0, nested_scroll_rows(10_000)),
            VirtualizationAxis::Vertical,
            96.0,
        )))
    }
}

fn virtual_button_rows(count: u64) -> Vec<SurfaceChild<()>> {
    (0..count)
        .map(|index| {
            SurfaceChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(28.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: radiant::layout::Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                SurfaceNode::widget(
                    ButtonWidget::new(
                        index + 10,
                        format!("Row {index:05}"),
                        WidgetSizing::fixed(Vector2::new(160.0, 28.0)),
                    ),
                    WidgetMessageMapper::none(),
                ),
            )
        })
        .collect()
}

fn nested_scroll_rows(count: u64) -> Vec<SurfaceChild<()>> {
    (0..count)
        .map(|index| {
            SurfaceChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(32.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: radiant::layout::Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                SurfaceNode::scroll_area(
                    20_000 + index,
                    SurfaceNode::column(40_000 + index, 0.0, nested_scroll_children(index)),
                ),
            )
        })
        .collect()
}

fn nested_scroll_children(row_index: u64) -> Vec<SurfaceChild<()>> {
    (0..4)
        .map(|child_index| {
            SurfaceChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(18.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: radiant::layout::Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                SurfaceNode::text(
                    80_000 + row_index * 4 + child_index,
                    format!("Nested {row_index:05}.{child_index}"),
                    WidgetSizing::fixed(Vector2::new(180.0, 18.0)),
                ),
            )
        })
        .collect()
}
