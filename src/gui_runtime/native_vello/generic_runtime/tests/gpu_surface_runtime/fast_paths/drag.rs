use super::super::*;
use crate::runtime::{DragPreview, DragRequest, ExternalDragRequest};

#[test]
fn active_runtime_drag_disables_gpu_surface_hover_fast_path() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge::default(),
        Vector2::new(240.0, 80.0),
    );
    runner.rebuild_scene();
    runner
        .core
        .runtime
        .execute_command(Command::begin_drag(DragRequest::new(
            DragPreview::sized("Loops", Vector2::new(150.0, 24.0)),
            Point::new(30.0, 20.0),
        )));

    let point = Point::new(40.0, 20.0);
    assert!(!runner.can_fast_path_native_hover_move(point));
    assert!(!runner.can_fast_path_gpu_surface_pointer_move(Some(point), Point::new(80.0, 20.0)));
}

#[test]
fn active_runtime_drag_defers_external_drag_launch_on_cursor_left() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge::default(),
        Vector2::new(240.0, 80.0),
    );
    runner
        .core
        .runtime
        .execute_command(Command::begin_external_drag_without_completion(
            ExternalDragRequest::files(
                [std::path::PathBuf::from(r"C:\samples\kick.wav")],
                "kick.wav",
            ),
        ));
    runner
        .core
        .runtime
        .execute_command(Command::begin_drag(DragRequest::new(
            DragPreview::sized("kick.wav", Vector2::new(150.0, 24.0)),
            Point::new(30.0, 20.0),
        )));

    let outcome = runner.launch_external_drag_if_armed();

    assert!(!outcome.needs_redraw());
    assert!(runner.core.runtime.external_drag_armed());
}
