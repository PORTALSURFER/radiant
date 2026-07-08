use super::{
    super::GenericNativeRuntimeCore,
    fixtures::{PaintOnlyPointerMoveBridge, QuietInteractiveRowBridge},
};
use crate::{
    layout::{Point, Vector2},
    runtime::{PaintFillRect, PaintPrimitive},
    theme::ThemeTokens,
};

#[test]
fn paint_only_pointer_move_overlay_skips_scene_rebuild_after_hover_enters() {
    let mut core =
        GenericNativeRuntimeCore::new(PaintOnlyPointerMoveBridge, Vector2::new(120.0, 40.0));
    let point = core
        .runtime
        .layout()
        .rects
        .get(&73)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("paint-only pointer widget should be laid out");

    let first = core.route_pointer_move(point);
    assert!(first.routed);
    assert!(first.needs_scene_rebuild());

    let second = core.route_pointer_move(Point::new(point.x + 11.0, point.y));
    assert!(second.routed);
    assert!(second.is_paint_only());
    assert!(!second.needs_scene_rebuild());
    assert!(second.needs_redraw());

    let mut overlay = Vec::new();
    core.runtime
        .runtime_overlay_paint_into(&ThemeTokens::default(), &mut overlay);
    assert!(
        overlay.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::FillRect(PaintFillRect { rect, .. })
                if (rect.center().x - point.x - 11.0).abs() < 0.01
        )),
        "paint-only pointer overlay should use the latest widget-local cursor position"
    );
}

#[test]
fn ordinary_interactive_row_hover_stays_off_model_refresh_after_enter() {
    let mut core = GenericNativeRuntimeCore::new(
        QuietInteractiveRowBridge::default(),
        Vector2::new(120.0, 40.0),
    );
    let point = core
        .runtime
        .layout()
        .rects
        .get(&85)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("quiet row should be laid out");

    let first = core.route_pointer_move(point);
    assert!(first.routed);
    assert!(first.needs_scene_rebuild());
    assert_eq!(core.runtime.bridge().project_count, 1);
    assert_eq!(core.runtime.bridge().update_count, 0);

    let second = core.route_pointer_move(Point::new(point.x + 11.0, point.y));
    assert!(second.routed);
    assert!(!second.needs_redraw());
    assert_eq!(
        core.runtime.bridge().project_count,
        1,
        "stable row hover must not refresh the host surface"
    );
    assert_eq!(
        core.runtime.bridge().update_count,
        0,
        "stable row hover must not emit host messages"
    );
}
