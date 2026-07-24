use crate::{
    gui::types::{Point, Rect, Vector2},
    runtime::{
        GpuSurfaceCapabilities, GpuSurfaceContent, PaintGpuSurface, PaintPrimitive,
        SurfacePaintPlan,
    },
    theme::ThemeTokens,
};
use std::sync::Arc;

#[test]
fn gpu_queries_return_matching_primitives_in_paint_order() {
    let theme = ThemeTokens::default();
    let mut plan = SurfacePaintPlan::empty(&theme);
    let gpu_rect = Rect::from_min_size(Point::new(4.0, 5.0), Vector2::new(32.0, 16.0));
    let atlas = crate::gui::types::ImageRgba::new(1, 1, vec![255, 255, 255, 255])
        .expect("valid test atlas");

    plan.primitives
        .push(PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id: 33,
            key: 1,
            revision: 2,
            rect: gpu_rect,
            content: GpuSurfaceContent::RgbaAtlas {
                atlas: Arc::new(atlas),
                source_rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
            },
            capabilities: GpuSurfaceCapabilities::default(),
            overlays: Vec::new(),
        }));

    assert_eq!(
        plan.gpu_surfaces()
            .map(|surface| surface.widget_id)
            .collect::<Vec<_>>(),
        vec![33]
    );
    assert_eq!(
        plan.render_canvases()
            .map(|surface| surface.widget_id)
            .collect::<Vec<_>>(),
        vec![33]
    );
    assert_eq!(
        plan.primitives[0].gpu_surface().map(|surface| surface.rect),
        Some(gpu_rect)
    );
    assert_eq!(
        plan.primitives[0]
            .render_canvas()
            .map(|surface| surface.rect),
        Some(gpu_rect)
    );
}
