use super::NativeVelloFrameState;
use crate::gui_runtime::native_vello::color_from_rgba;
use std::time::{Duration, Instant};
use tracing::error;
use vello::{AaConfig, RenderParams, Renderer, Scene, kurbo::Affine, util::RenderSurface, wgpu};
use winit::event_loop::ActiveEventLoop;

pub(super) fn render_scene_texture_if_needed(
    frame: &mut NativeVelloFrameState,
    renderer: &mut Renderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    surface: &RenderSurface<'_>,
    dpi_scale: crate::theme::DpiScale,
    event_loop: &ActiveEventLoop,
) -> Option<Duration> {
    if !frame.scene_texture_dirty {
        return Some(Duration::ZERO);
    }

    let render_started = Instant::now();
    let mut scaled_scene = Scene::new();
    let scene = if dpi_scale == crate::theme::DpiScale::ONE {
        &frame.scene
    } else {
        scaled_scene.append(&frame.scene, Some(Affine::scale(dpi_scale.factor() as f64)));
        &scaled_scene
    };
    let result = renderer.render_to_texture(
        device,
        queue,
        scene,
        &surface.target_view,
        &RenderParams {
            base_color: color_from_rgba(frame.last_paint_plan.clear_color),
            width: surface.config.width,
            height: surface.config.height,
            antialiasing_method: AaConfig::Area,
        },
    );
    let elapsed = render_started.elapsed();
    if let Err(err) = result {
        error!(
            "radiant generic native vello: render_to_texture failed: {:?}",
            err
        );
        event_loop.exit();
        return None;
    }

    frame.scene_texture_dirty = false;
    frame.mark_composited_base_dirty();
    Some(elapsed)
}
