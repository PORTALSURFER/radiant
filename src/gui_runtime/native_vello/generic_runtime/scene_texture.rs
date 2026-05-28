use super::NativeVelloFrameState;
use crate::gui_runtime::native_vello::color_from_rgba;
use std::time::{Duration, Instant};
use tracing::error;
use vello::{AaConfig, RenderParams, Renderer, util::RenderSurface, wgpu};
use winit::event_loop::ActiveEventLoop;

pub(super) struct SceneTextureContext<'a> {
    pub(super) renderer: &'a mut Renderer,
    pub(super) device: &'a wgpu::Device,
    pub(super) queue: &'a wgpu::Queue,
    pub(super) surface: &'a RenderSurface<'a>,
    pub(super) dpi_scale: crate::theme::DpiScale,
    pub(super) record_timing: bool,
    pub(super) event_loop: &'a ActiveEventLoop,
}

struct SceneTextureTarget<'a> {
    view: &'a wgpu::TextureView,
    width: u32,
    height: u32,
}

pub(super) fn render_scene_texture_if_needed(
    frame: &mut NativeVelloFrameState,
    context: &mut SceneTextureContext<'_>,
) -> Option<Duration> {
    if !frame.scene_texture_dirty {
        return Some(Duration::ZERO);
    }

    let elapsed = render_scene_to_view(
        frame,
        context,
        SceneTextureTarget {
            view: &context.surface.target_view,
            width: context.surface.config.width,
            height: context.surface.config.height,
        },
    )?;

    frame.scene_texture_dirty = false;
    frame.mark_composited_base_dirty();
    Some(elapsed)
}

pub(super) fn render_scene_to_surface_view(
    frame: &mut NativeVelloFrameState,
    context: &mut SceneTextureContext<'_>,
    surface_view: &wgpu::TextureView,
) -> Option<Duration> {
    render_scene_to_view(
        frame,
        context,
        SceneTextureTarget {
            view: surface_view,
            width: context.surface.config.width,
            height: context.surface.config.height,
        },
    )
}

fn render_scene_to_view(
    frame: &mut NativeVelloFrameState,
    context: &mut SceneTextureContext<'_>,
    target: SceneTextureTarget<'_>,
) -> Option<Duration> {
    let render_started = context.record_timing.then(Instant::now);
    let base_color = color_from_rgba(frame.last_paint_plan.clear_color);
    let scene = frame.scene_for_dpi_scale(context.dpi_scale);
    let result = context.renderer.render_to_texture(
        context.device,
        context.queue,
        scene,
        target.view,
        &RenderParams {
            base_color,
            width: target.width,
            height: target.height,
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
        context.event_loop.exit();
        return None;
    }

    Some(elapsed)
}
