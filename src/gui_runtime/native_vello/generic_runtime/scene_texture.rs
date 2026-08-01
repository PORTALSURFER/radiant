use super::submission_completion::NativeSubmissionCompletionWitness;
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
    pub(super) completion_witness: &'a mut NativeSubmissionCompletionWitness,
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

const OPAQUE_RENDERER_PANIC_MESSAGE: &str = "renderer panic payload was not a string";

fn catch_renderer_unwind<T>(render: impl FnOnce() -> T) -> Result<T, String> {
    // The production closure contains only the renderer call. AssertUnwindSafe is
    // intentional because a caught unwind returns through redraw's existing
    // terminal-cause/event-loop-exit path: this renderer is terminally abandoned,
    // and no later renderer call or retry is permitted in that redraw.
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(render))
        .map_err(normalize_renderer_panic_payload)
}

fn normalize_renderer_panic_payload(payload: Box<dyn std::any::Any + Send>) -> String {
    match payload.downcast::<String>() {
        Ok(message) => *message,
        Err(payload) => match payload.downcast::<&'static str>() {
            Ok(message) => (*message).to_owned(),
            Err(_) => OPAQUE_RENDERER_PANIC_MESSAGE.to_owned(),
        },
    }
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
    let render_params = RenderParams {
        base_color,
        width: target.width,
        height: target.height,
        antialiasing_method: AaConfig::Area,
    };
    let render_result = catch_renderer_unwind(|| {
        context.renderer.render_to_texture(
            context.device,
            context.queue,
            scene,
            target.view,
            &render_params,
        )
    });
    let result = match render_result {
        Ok(result) => result,
        Err(message) => {
            error!("radiant generic native vello: render_to_texture panicked: {message}");
            context.completion_witness.record_indeterminate_submission();
            return Err(NativeGenericRunError::FrameRender(message));
        }
    };
    let elapsed = render_started
        .map(|started| started.elapsed())
        .unwrap_or_default();
    if let Err(err) = result {
        let message = err.to_string();
        error!("radiant generic native vello: render_to_texture failed: {message}");
        context.completion_witness.record_indeterminate_submission();
        return Err(frame_render_error(message));
    }

    context.completion_witness.record_successful_submission();

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
    use super::{
        OPAQUE_RENDERER_PANIC_MESSAGE, catch_renderer_unwind, commit_scene_texture_render,
        frame_render_error,
    };
    use crate::gui_runtime::native_vello::NativeTextRenderer;
    use crate::runtime::RetainedSurfaceCachePolicy;
    use std::cell::Cell;
    use std::time::Duration;

    #[test]
    fn renderer_panic_boundary_returns_success_and_invokes_once() {
        let invocations = Cell::new(0);

        let value = catch_renderer_unwind(|| {
            invocations.set(invocations.get() + 1);
            42_u32
        })
        .expect("successful renderer calls should pass through");

        assert_eq!(value, 42);
        assert_eq!(invocations.get(), 1);
    }

    #[test]
    fn backend_result_error_maps_to_stable_frame_render_display() {
        let backend_result: Result<(), &str> = Err("backend rejected scene");
        let error = backend_result
            .map_err(frame_render_error)
            .expect_err("backend errors should map to FrameRender");

        assert_eq!(
            error,
            super::NativeGenericRunError::FrameRender("backend rejected scene".to_owned())
        );
        assert_eq!(
            error.to_string(),
            "native frame rendering failed: backend rejected scene"
        );
    }

    #[test]
    fn static_renderer_panic_is_caught_as_owned_text() {
        let message = catch_renderer_unwind(|| -> () {
            std::panic::panic_any("static renderer panic");
        })
        .expect_err("static renderer panics should be contained");
        let error = super::NativeGenericRunError::FrameRender(message);

        assert_eq!(
            error.to_string(),
            "native frame rendering failed: static renderer panic"
        );
    }

    #[test]
    fn owned_renderer_panic_is_retained_after_payload_lifetime_ends() {
        let message = catch_renderer_unwind(|| -> () {
            std::panic::panic_any(String::from("owned renderer panic"));
        })
        .expect_err("owned renderer panics should be contained");
        let error = super::NativeGenericRunError::FrameRender(message);

        assert_eq!(
            error.to_string(),
            "native frame rendering failed: owned renderer panic"
        );
    }

    #[test]
    fn opaque_renderer_panic_uses_deterministic_fallback() {
        struct OpaquePayload;

        let message = catch_renderer_unwind(|| -> () {
            std::panic::panic_any(OpaquePayload);
        })
        .expect_err("opaque renderer panics should be contained");

        assert_eq!(message, OPAQUE_RENDERER_PANIC_MESSAGE);
    }

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
