use super::{NativeGenericRunError, NativeVelloFrameState};
use crate::gui_runtime::native_vello::color_from_rgba;
use std::{
    fmt,
    time::{Duration, Instant},
};
use tracing::error;
use vello::{AaConfig, RenderParams, Renderer, util::RenderSurface, wgpu};

pub(super) struct SceneTextureContext<'a> {
    pub(super) renderer: &'a mut Renderer,
    pub(super) device: &'a wgpu::Device,
    pub(super) queue: &'a wgpu::Queue,
    pub(super) surface: &'a RenderSurface<'a>,
    pub(super) dpi_scale: crate::theme::DpiScale,
    pub(super) record_timing: bool,
}

struct SceneTextureTarget<'a> {
    view: &'a wgpu::TextureView,
    width: u32,
    height: u32,
}

pub(super) fn render_scene_texture_if_needed(
    frame: &mut NativeVelloFrameState,
    context: &mut SceneTextureContext<'_>,
) -> Result<Duration, NativeGenericRunError> {
    if !frame.scene_texture_dirty {
        return Ok(Duration::ZERO);
    }

    let result = render_scene_to_view(
        frame,
        context,
        SceneTextureTarget {
            view: &context.surface.target_view,
            width: context.surface.config.width,
            height: context.surface.config.height,
        },
    );
    let elapsed = commit_scene_texture_render(frame, result)?;

    Ok(elapsed)
}

pub(super) fn render_scene_to_surface_view(
    frame: &mut NativeVelloFrameState,
    context: &mut SceneTextureContext<'_>,
    surface_view: &wgpu::TextureView,
) -> Result<Duration, NativeGenericRunError> {
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
) -> Result<Duration, NativeGenericRunError> {
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
        let message = err.to_string();
        error!("radiant generic native vello: render_to_texture failed: {message}");
        return Err(frame_render_error(message));
    }

    Ok(elapsed)
}

fn commit_scene_texture_render(
    frame: &mut NativeVelloFrameState,
    result: Result<Duration, NativeGenericRunError>,
) -> Result<Duration, NativeGenericRunError> {
    let elapsed = result?;
    frame.scene_texture_dirty = false;
    frame.mark_composited_base_dirty();
    Ok(elapsed)
}

pub(super) fn frame_render_error(error: impl fmt::Display) -> NativeGenericRunError {
    NativeGenericRunError::FrameRender(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::super::NativeVelloFrameState;
    use super::{commit_scene_texture_render, frame_render_error};
    use crate::gui_runtime::native_vello::NativeTextRenderer;
    use crate::runtime::RetainedSurfaceCachePolicy;
    use std::time::Duration;

    #[test]
    fn frame_render_error_owns_backend_message_and_has_stable_display() {
        let detail = String::from("backend rejected scene");
        let error = frame_render_error(detail.as_str());
        drop(detail);

        assert_eq!(
            error.to_string(),
            "native frame rendering failed: backend rejected scene"
        );
    }

    #[test]
    fn failed_scene_texture_render_does_not_commit_dirty_state() {
        let mut frame = NativeVelloFrameState::new(
            NativeTextRenderer::new(),
            RetainedSurfaceCachePolicy::default(),
        );
        frame.composited_base_dirty = false;
        let failure = frame_render_error("backend rejected scene");

        assert_eq!(
            commit_scene_texture_render(&mut frame, Err(failure.clone())),
            Err(failure)
        );
        assert!(frame.scene_texture_dirty);
        assert!(!frame.composited_base_dirty);
    }

    #[test]
    fn successful_scene_texture_render_commits_existing_success_behavior() {
        let mut frame = NativeVelloFrameState::new(
            NativeTextRenderer::new(),
            RetainedSurfaceCachePolicy::default(),
        );
        frame.composited_base_dirty = false;

        assert_eq!(
            commit_scene_texture_render(&mut frame, Ok(Duration::from_millis(2))),
            Ok(Duration::from_millis(2))
        );
        assert!(!frame.scene_texture_dirty);
        assert!(frame.composited_base_dirty);
    }
}
