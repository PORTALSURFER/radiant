use crate::{
    gui::types::{Point, Vector2},
    gui_runtime::NativeRunOptions,
    gui_runtime::native_vello::run_native_vello_runtime,
    layout::{ContainerPolicy, SlotParams},
    runtime::{RuntimeBridge, SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper},
    widgets::{ButtonWidget, PointerButton, WidgetSizing, WidgetSpec},
};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
enum DemoMessage {
    Activate,
}

#[derive(Default)]
struct DemoState {
    activations: usize,
}

#[test]
fn generic_native_entrypoint_accepts_runtime_bridge_without_shell_types() {
    let _entrypoint: fn(NativeRunOptions, DemoBridge) -> Result<(), String> =
        run_native_vello_runtime::<DemoBridge, DemoMessage>;
}

#[test]
fn generic_native_core_routes_pointer_messages() {
    let mut core = super::super::super::generic_runtime::GenericNativeRuntimeCore::new(
        DemoBridge::default(),
        Vector2::new(160.0, 40.0),
    );
    let point = core
        .runtime
        .layout()
        .rects
        .get(&11)
        .map(|rect| Point::new(rect.min.x + 1.0, rect.min.y + 1.0))
        .expect("button should be laid out");

    assert!(
        core.route_pointer_press(point, PointerButton::Primary)
            .routed
    );
    assert!(
        core.route_pointer_release(point, PointerButton::Primary)
            .routed
    );
    assert_eq!(core.runtime.bridge().state.activations, 1);
}

#[derive(Default)]
struct DemoBridge {
    state: DemoState,
}

impl RuntimeBridge<DemoMessage> for DemoBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        let button = WidgetSpec::Button(ButtonWidget::new(
            11,
            "Activate",
            WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
        ));
        Arc::new(UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            vec![SurfaceChild::new(
                SlotParams::fill(),
                SurfaceNode::widget(
                    button,
                    WidgetMessageMapper::button(|_| DemoMessage::Activate),
                ),
            )],
        )))
    }

    fn reduce_message(&mut self, message: DemoMessage) {
        match message {
            DemoMessage::Activate => self.state.activations += 1,
        }
    }
}
