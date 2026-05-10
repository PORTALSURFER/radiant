//! Small route, cadence, and profiling helpers for the generic native runner.

use super::GenericRouteOutcome;
use crate::{
    layout::{Rect, Vector2},
    runtime::PaintPrimitive,
};
use std::time::Duration;
use tracing::info;
use winit::event::MouseScrollDelta;

pub(super) fn fast_pointer_move_gpu_surface_hit_rects(primitives: &[PaintPrimitive]) -> Vec<Rect> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::GpuSurface(surface) if surface.capabilities.fast_pointer_move => {
                Some(surface.rect)
            }
            _ => None,
        })
        .collect()
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
        repaint_requested = outcome.repaint_requested,
        "radiant native input profile"
    );
}

pub(super) fn render_profile_enabled() -> bool {
    std::env::var("RADIANT_NATIVE_RENDER_PROFILE")
        .ok()
        .is_some_and(|value| crate::env_flags::is_truthy(&value))
}
