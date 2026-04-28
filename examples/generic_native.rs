//! Standalone native Radiant example using the generic runtime surface.

use radiant::{
    layout::{ContainerKind, ContainerPolicy, SlotParams, Vector2},
    runtime::{
        NativeRunOptions, SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper,
        declarative_runtime_bridge, run_native_vello_runtime,
    },
    widgets::{ButtonMessage, ButtonWidget, TextWidget, WidgetSizing, WidgetSpec},
};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
enum DemoMessage {
    Increment,
}

#[derive(Default)]
struct DemoState {
    count: usize,
}

fn main() -> Result<(), String> {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
        },
    );

    run_native_vello_runtime(
        NativeRunOptions {
            title: String::from("Radiant Generic Native Example"),
            inner_size: Some([320.0, 96.0]),
            min_inner_size: Some([240.0, 80.0]),
            ..NativeRunOptions::default()
        },
        bridge,
    )
}

fn project_surface(state: &mut DemoState) -> Arc<UiSurface<DemoMessage>> {
    let title = WidgetSpec::Text(TextWidget::new(
        10,
        format!("Generic Radiant count: {}", state.count),
        WidgetSizing::fixed(Vector2::new(180.0, 24.0)).with_baseline(16.0),
    ));
    let button = WidgetSpec::Button(ButtonWidget::new(
        11,
        "Increment",
        WidgetSizing::fixed(Vector2::new(96.0, 32.0)),
    ));

    Arc::new(UiSurface::new(SurfaceNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Row,
            spacing: 12.0,
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
                    WidgetMessageMapper::button(|message| match message {
                        ButtonMessage::Activate => DemoMessage::Increment,
                    }),
                ),
            ),
        ],
    )))
}
