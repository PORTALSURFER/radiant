use super::*;

pub(super) fn render_scene_texture_if_needed(
    frame: &mut NativeVelloFrameState,
    renderer: &mut Renderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    surface: &RenderSurface<'_>,
    event_loop: &ActiveEventLoop,
) -> Option<Duration> {
    if !frame.scene_texture_dirty {
        return Some(Duration::ZERO);
    }

    let render_started = Instant::now();
    let result = renderer.render_to_texture(
        device,
        queue,
        &frame.scene,
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
