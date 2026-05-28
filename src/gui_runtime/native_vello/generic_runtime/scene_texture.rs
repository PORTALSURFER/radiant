use super::NativeVelloFrameState;
use crate::gui_runtime::native_vello::color_from_rgba;
use std::time::{Duration, Instant};
use tracing::error;
use vello::{AaConfig, RenderParams, Renderer, util::RenderSurface, wgpu};
use winit::event_loop::ActiveEventLoop;

pub(super) fn render_scene_texture_if_needed(
    frame: &mut NativeVelloFrameState,
    renderer: &mut Renderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    surface: &RenderSurface<'_>,
    dpi_scale: crate::theme::DpiScale,
    record_timing: bool,
    event_loop: &ActiveEventLoop,
) -> Option<Duration> {
    if !frame.scene_texture_dirty {
        return Some(Duration::ZERO);
    }

    let elapsed = render_scene_to_view(
        frame,
        renderer,
        device,
        queue,
        &surface.target_view,
        surface.config.width,
        surface.config.height,
        dpi_scale,
        record_timing,
        event_loop,
    )?;

    frame.scene_texture_dirty = false;
    frame.mark_composited_base_dirty();
    Some(elapsed)
}

pub(super) fn render_scene_to_surface_view(
    frame: &mut NativeVelloFrameState,
    renderer: &mut Renderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    surface: &RenderSurface<'_>,
    surface_view: &wgpu::TextureView,
    dpi_scale: crate::theme::DpiScale,
    record_timing: bool,
    event_loop: &ActiveEventLoop,
) -> Option<Duration> {
    render_scene_to_view(
        frame,
        renderer,
        device,
        queue,
        surface_view,
        surface.config.width,
        surface.config.height,
        dpi_scale,
        record_timing,
        event_loop,
    )
}

fn render_scene_to_view(
    frame: &mut NativeVelloFrameState,
    renderer: &mut Renderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    target_view: &wgpu::TextureView,
    width: u32,
    height: u32,
    dpi_scale: crate::theme::DpiScale,
    record_timing: bool,
    event_loop: &ActiveEventLoop,
) -> Option<Duration> {
    let render_started = record_timing.then(Instant::now);
    let base_color = color_from_rgba(frame.last_paint_plan.clear_color);
    let scene = frame.scene_for_dpi_scale(dpi_scale);
    let result = renderer.render_to_texture(
        device,
        queue,
        scene,
        target_view,
        &RenderParams {
            base_color,
            width,
            height,
            antialiasing_method: AaConfig::Area,
        },
    );
    let elapsed = render_started
        .map(|started| started.elapsed())
        .unwrap_or_default();
    if let Err(err) = result {
        error!(
            "radiant generic native vello: render_to_texture failed: {:?}",
            err
        );
        event_loop.exit();
        return None;
    }

    Some(elapsed)
}
