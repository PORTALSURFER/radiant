//! Small route, cadence, and profiling helpers for the generic native runner.

use super::GenericRouteOutcome;
use crate::{
    layout::{Rect, Vector2},
    runtime::PaintPrimitive,
};
use std::time::Duration;
use tracing::info;
use winit::event::MouseScrollDelta;

pub(super) fn collect_fast_pointer_move_gpu_surface_hit_rects(
    primitives: &[PaintPrimitive],
    hit_rects: &mut Vec<Rect>,
) {
    hit_rects.clear();
    hit_rects.extend(primitives.iter().filter_map(|primitive| match primitive {
        PaintPrimitive::GpuSurface(surface) if surface.capabilities.fast_pointer_move => {
            Some(surface.rect)
        }
        _ => None,
    }));
}

pub(super) fn animation_frame_interval(target_fps: u32) -> Duration {
    let fps = target_fps.clamp(1, 240);
    Duration::from_secs_f64(1.0 / f64::from(fps))
}

pub(super) fn scroll_delta_to_logical(delta: MouseScrollDelta) -> Vector2 {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => Vector2::new(-(x * 40.0), -(y * 40.0)),
        MouseScrollDelta::PixelDelta(position) => {
            Vector2::new(-(position.x as f32), -(position.y as f32))
        }
    }
}

pub(super) fn maybe_log_route_profile(
    reason: &'static str,
    elapsed: Duration,
    outcome: GenericRouteOutcome,
) {
    if !render_profile_enabled() {
        return;
    }
    info!(
        reason,
        event_route_us = elapsed.as_micros(),
        routed = outcome.routed,
        redraw_requested = outcome.redraw_requested,
        repaint_requested = outcome.repaint_requested,
        "radiant native input profile"
    );
}

pub(super) fn render_profile_enabled() -> bool {
    std::env::var("RADIANT_NATIVE_RENDER_PROFILE")
        .ok()
        .is_some_and(|value| crate::env_flags::is_truthy(&value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::ImageRgba;
    use crate::runtime::{GpuSurfaceCapabilities, PaintGpuSurface};
    use std::sync::Arc;

    #[test]
    fn gpu_surface_hit_rect_collection_reuses_existing_buffer() {
        let mut hit_rects = Vec::with_capacity(8);
        hit_rects.push(Rect::from_min_size(
            crate::layout::Point::new(99.0, 99.0),
            Vector2::new(1.0, 1.0),
        ));
        let initial_capacity = hit_rects.capacity();
        let rect = Rect::from_min_size(crate::layout::Point::new(1.0, 2.0), Vector2::new(3.0, 4.0));
        let ignored_rect =
            Rect::from_min_size(crate::layout::Point::new(5.0, 6.0), Vector2::new(7.0, 8.0));
        let surface = PaintGpuSurface {
            widget_id: 7,
            key: 7,
            revision: 1,
            rect,
            content: crate::runtime::GpuSurfaceContent::RgbaAtlas {
                source_rect: rect,
                atlas: Arc::new(ImageRgba::new(1, 1, vec![255; 4]).expect("valid image")),
            },
            capabilities: GpuSurfaceCapabilities {
                fast_pointer_move: true,
                coalesce_vertical_wheel: false,
                native_hover_cursor: None,
            },
            overlays: Vec::new(),
        };
        let mut ignored_surface = surface.clone();
        ignored_surface.rect = ignored_rect;
        ignored_surface.capabilities.fast_pointer_move = false;
        let primitives = [
            PaintPrimitive::GpuSurface(ignored_surface),
            PaintPrimitive::GpuSurface(surface),
        ];

        collect_fast_pointer_move_gpu_surface_hit_rects(&primitives, &mut hit_rects);

        assert_eq!(hit_rects, [rect]);
        assert_eq!(hit_rects.capacity(), initial_capacity);
    }
}
