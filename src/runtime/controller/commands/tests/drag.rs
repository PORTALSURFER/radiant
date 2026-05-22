use super::fixtures::QueuedCommandBridge;
use crate::{
    gui::types::{Point, Vector2},
    runtime::{Command, DragPreview, DragRequest, Event, PaintPrimitive, SurfaceRuntime},
    theme::ThemeTokens,
};

#[test]
fn drag_command_paints_runtime_preview_and_tracks_pointer_move() {
    let mut runtime =
        SurfaceRuntime::new(QueuedCommandBridge::default(), Vector2::new(320.0, 200.0));
    let outcome = runtime.execute_command(Command::begin_drag(DragRequest::new(
        DragPreview::sized("Loops", Vector2::new(150.0, 24.0)),
        Point::new(20.0, 30.0),
    )));

    assert!(runtime.drag_session_active());
    assert!(outcome.surface_repaint_requested);

    let mut plan = runtime.paint_plan(&ThemeTokens::default());
    assert!(
        plan.primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::Text(text) if text.text.as_str() == "Loops"
        )),
        "active drag should paint its preview label"
    );

    runtime.dispatch_event(Event::PointerMove {
        position: Point::new(80.0, 90.0),
    });
    plan = runtime.paint_plan(&ThemeTokens::default());
    assert!(
        plan.primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::FillRect(fill)
                if fill.rect.min.x == 94.0 && fill.rect.min.y == 108.0
        )),
        "drag preview should follow pointer using the runtime preview offset"
    );

    assert!(runtime.hide_drag_preview_for_cursor_left());
    plan = runtime.paint_plan(&ThemeTokens::default());
    assert!(
        !plan.primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::Text(text) if text.text.as_str() == "Loops"
        )),
        "drag preview should hide when the pointer leaves the surface"
    );

    runtime.dispatch_event(Event::PointerMove {
        position: Point::new(30.0, 40.0),
    });
    plan = runtime.paint_plan(&ThemeTokens::default());
    assert!(
        plan.primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::Text(text) if text.text.as_str() == "Loops"
        )),
        "drag preview should reappear when the pointer re-enters"
    );

    runtime.execute_command(Command::end_drag());
    assert!(!runtime.drag_session_active());
}
