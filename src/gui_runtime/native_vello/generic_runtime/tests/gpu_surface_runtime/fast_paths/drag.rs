use super::super::*;
use crate::runtime::{DragPreview, DragRequest, ExternalDragRequest};
use crate::widgets::PointerModifiers;

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
fn active_runtime_drag_can_transfer_to_external_drag() {
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
    runner.rebuild_scene();
    runner
        .core
        .runtime
        .execute_command(Command::begin_drag(DragRequest::new(
            DragPreview::sized("kick.wav", Vector2::new(150.0, 24.0)),
            Point::new(30.0, 20.0),
        )));
    runner.core.route_pointer_press_with_modifiers(
        Point::new(30.0, 20.0),
        PointerButton::Primary,
        PointerModifiers::default(),
    );

    let session = runner
        .core
        .runtime
        .take_external_drag_session()
        .expect("external drag session should remain armed until native launch");
    runner.core.runtime.cancel_pointer_capture();
    let preview_cleared = runner.core.runtime.take_drag_preview_for_external_drag();

    assert!(preview_cleared);
    assert!(!runner.core.runtime.drag_session_active());
    assert!(runner.core.runtime.pointer_capture().is_none());
    assert_eq!(session.request.preview.label, "kick.wav");
}
