//! Minimal hello-world app built on the generic Radiant runtime.

use radiant::{
    layout::Vector2,
    runtime::{
        declarative_command_runtime_bridge, run_native_vello_runtime, Command, NativeRunOptions,
        SurfaceChild, SurfaceNode, UiSurface,
    },
    widgets::WidgetSizing,
};
use std::sync::Arc;

fn main() -> Result<(), String> {
    let bridge = declarative_command_runtime_bridge((), project_surface, |(), ()| Command::none());

    run_native_vello_runtime(
        NativeRunOptions {
            title: String::from("Radiant Hello World"),
            inner_size: Some([320.0, 120.0]),
            min_inner_size: Some([240.0, 96.0]),
            ..NativeRunOptions::default()
        },
        bridge,
    )
}

fn project_surface((): &mut ()) -> Arc<UiSurface<()>> {
    Arc::new(UiSurface::new(SurfaceNode::row(
        1,
        0.0,
        vec![SurfaceChild::fill(SurfaceNode::text(
            2,
            "Hello, world!",
            WidgetSizing::fixed(Vector2::new(160.0, 32.0)).with_baseline(22.0),
        ))],
    )))
}
