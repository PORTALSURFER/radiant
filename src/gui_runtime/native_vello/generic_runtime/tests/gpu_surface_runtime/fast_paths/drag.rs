use super::super::*;
use crate::runtime::{DragPreview, DragRequest, ExternalDragRequest, PaintPrimitive};
use crate::widgets::PointerModifiers;

#[test]
fn active_runtime_drag_moves_through_transient_overlay_without_scene_rebuild() {
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

    runner.rebuild_scene();
    assert!(
        !contains_drag_label(&runner.frame.last_paint_plan.primitives, "Loops"),
        "native cached base scene should not retain drag preview positions"
    );
    runner.frame.scene_texture_dirty = false;
    runner.frame.composited_base_dirty = false;

    let first_point = Point::new(40.0, 20.0);
    let first_move = runner.core.route_pointer_move(first_point);
    assert!(first_move.needs_scene_rebuild());
    runner.handle_gpu_surface_pointer_move_outcome(
        first_move,
        Some(Point::new(30.0, 20.0)),
        first_point,
    );
    runner.frame.scene_texture_dirty = false;
    runner.frame.composited_base_dirty = false;

    let point = Point::new(80.0, 20.0);
    let moved = runner.core.route_pointer_move(point);
    assert!(moved.paint_only_requested);
    assert!(!moved.needs_scene_rebuild());
    runner.handle_gpu_surface_pointer_move_outcome(moved, Some(first_point), point);
    runner.paint_transient_overlays(&mut RenderFrameProfile::default());

    assert!(
        !runner.frame.scene_texture_dirty,
        "drag preview motion should not rebuild the Vello scene"
    );
    assert!(
        !runner.frame.composited_base_dirty,
        "drag preview motion should keep the cached base frame"
    );
    assert!(
        contains_drag_label(&runner.frame.transient_overlay_primitives, "Loops"),
        "drag preview should be redrawn as a transient runtime overlay"
    );
    assert!(!runner.can_fast_path_native_hover_move(point));
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

#[test]
fn focus_loss_cleans_native_pointer_state_before_external_drag_launch() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge::default(),
        Vector2::new(240.0, 80.0),
    );
    runner.rebuild_scene();
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
    runner.core.route_pointer_press_with_modifiers(
        Point::new(30.0, 20.0),
        PointerButton::Primary,
        PointerModifiers::default(),
    );
    runner.input.last_cursor = Some(Point::new(60.0, 20.0));
    assert!(runner.update_gpu_surface_cursor_overlay(Point::new(60.0, 20.0)));

    let outcome = runner.handle_focus_lost_before_external_drag();

    assert!(outcome.needs_redraw());
    assert!(runner.core.runtime.external_drag_armed());
    assert!(runner.core.runtime.pointer_capture().is_none());
    assert_eq!(runner.input.last_cursor, None);
    let session = runner
        .core
        .runtime
        .take_external_drag_session()
        .expect("external drag should stay armed for native launch");
    assert_eq!(session.request.preview.label, "kick.wav");
    let surface = runner
        .frame
        .last_paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::GpuSurface(surface) => Some(surface),
            _ => None,
        })
        .expect("gpu surface primitive");
    assert!(
        !surface
            .overlays
            .iter()
            .any(|overlay| matches!(overlay, GpuSurfaceOverlay::RuntimeVerticalLine { .. }))
    );
}

fn contains_drag_label(primitives: &[PaintPrimitive], label: &str) -> bool {
    primitives.iter().any(|primitive| {
        matches!(
            primitive,
            PaintPrimitive::Text(text) if text.text.as_str() == label
        )
    })
}
