use super::super::*;
use crate::runtime::PaintFillRect;
use std::sync::Arc;

#[test]
fn native_routed_paint_only_pointer_move_skips_scene_rebuild() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        PaintOnlyPointerBridge,
        Vector2::new(160.0, 48.0),
    );
    runner.rebuild_scene();
    let first = Point::new(8.0, 8.0);
    let second = Point::new(32.0, 8.0);

    let enter = runner.core.route_pointer_move(first);
    assert!(enter.routed);
    assert!(enter.needs_scene_rebuild());
    runner.handle_gpu_surface_pointer_move_outcome(enter, None, first);
    assert!(
        runner.frame.scene_texture_dirty,
        "initial hover enter still rebuilds the base scene"
    );
    runner.frame.scene_texture_dirty = false;
    runner.frame.composited_base_dirty = false;

    let moved = runner.core.route_pointer_move(second);
    assert!(moved.routed);
    assert!(moved.is_paint_only());
    assert!(!moved.needs_scene_rebuild());
    runner.handle_gpu_surface_pointer_move_outcome(moved, Some(first), second);

    assert!(
        !runner.frame.scene_texture_dirty,
        "routed paint-only pointer motion should not rebuild the Vello scene"
    );
    assert!(
        !runner.frame.composited_base_dirty,
        "routed paint-only pointer motion should keep the cached base frame"
    );
}

struct PaintOnlyPointerBridge;

impl RuntimeBridge<()> for PaintOnlyPointerBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::custom_widget(
            PaintOnlyPointerWidget::new(),
            WidgetMessageMapper::none(),
        )))
    }

    fn reduce_message(&mut self, _message: ()) {}
}

#[derive(Clone, Debug)]
struct PaintOnlyPointerWidget {
    common: WidgetCommon,
    last_position: Point,
}

impl PaintOnlyPointerWidget {
    fn new() -> Self {
        Self {
            common: WidgetCommon::new(91, WidgetSizing::fixed(Vector2::new(120.0, 32.0)))
                .with_pointer_focus(),
            last_position: Point::new(0.0, 0.0),
        }
    }
}

impl crate::widgets::WidgetPointerMotion for PaintOnlyPointerWidget {
    fn revision(&self) -> crate::widgets::WidgetPointerMotionRevision {
        crate::widgets::WidgetPointerMotionRevision::exact(true)
    }

    fn prefers_pointer_move_paint_only(&self) -> bool {
        true
    }

    fn pointer_move_overlay_is_valid(&self) -> bool {
        true
    }
}

impl Widget for PaintOnlyPointerWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn capabilities(&self) -> crate::widgets::WidgetCapabilities<'_> {
        crate::widgets::WidgetCapabilities::none()
    }

    fn capabilities_v2(&self) -> crate::widgets::WidgetCapabilitiesV2<'_> {
        crate::widgets::WidgetCapabilitiesV2::new().with_pointer_motion(self)
    }

    fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        if let WidgetInput::PointerMove { position, .. } = input {
            self.last_position = position;
        }
        None
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &crate::layout::LayoutOutput,
        _theme: &crate::theme::ThemeTokens,
    ) {
    }

    fn append_runtime_overlay_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &crate::layout::LayoutOutput,
        theme: &crate::theme::ThemeTokens,
    ) {
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: Rect::from_min_max(
                Point::new(self.last_position.x - 1.0, bounds.min.y),
                Point::new(self.last_position.x + 1.0, bounds.max.y),
            ),
            color: theme.highlight_orange,
        }));
    }
}
