//! Generic `RuntimeBridge` native Vello runner.

use super::*;
use crate::gui::repaint::RepaintSignal;

mod composited_base;
mod core;
mod gpu_surface;
mod gpu_surface_cursor;
mod gpu_surface_interaction;
mod input;
mod keyboard;
mod lifecycle;
mod post_gpu_overlay;
mod present;
mod runner;
mod runtime_helpers;
mod runtime_wakeup;
mod scene;
mod surface;
mod window;

use composited_base::CompositedBaseFrame;
pub(in crate::gui_runtime::native_vello) use core::{
    GenericNativeRuntimeCore, GenericRouteOutcome,
};
use gpu_surface::GpuSurfaceRenderer;
use gpu_surface_interaction::PendingGpuSurfaceWheel;
use input::{key_code_from_winit, keypress_from_input, pointer_button_from_winit};
use post_gpu_overlay::PostGpuOverlayRenderer;
use present::RenderFrameProfile;
use runner::GenericNativeVelloRunner;
use runtime_helpers::{
    GpuSurfaceInteractionRegion, animation_frame_interval, collect_gpu_surface_interaction_regions,
    maybe_log_route_profile, render_profile_enabled, scroll_delta_to_logical,
};
use runtime_wakeup::RuntimeWakeup;
pub(in crate::gui_runtime::native_vello) use scene::{
    RetainedSurfaceEncodeStats, RetainedSurfaceFrameCache, SceneTextRunBuffer,
    SurfaceSceneEncodeContext, encode_surface_paint_plan_to_scene,
};
use window::generic_window_attributes;

struct GenericSharedPixelBytes(Arc<[u8]>);

impl AsRef<[u8]> for GenericSharedPixelBytes {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

/// Run a generic [`RuntimeBridge`] through the native Vello backend.
///
/// This entrypoint is intentionally narrower than the compatibility
/// shell runner: it renders public `UiSurface` paint primitives, routes pointer
/// and keyboard input into projected widget ids, reduces host-defined messages,
/// and requests redraws when the surface changes.
pub fn run_native_vello_runtime<Bridge, Message>(
    options: NativeRunOptions,
    bridge: Bridge,
) -> Result<(), String>
where
    Bridge: RuntimeBridge<Message> + 'static,
    Message: 'static,
{
    run_native_vello_runtime_with_artifacts(options, bridge)
        .result
        .map_err(|err| err.to_string())
}

/// Run a generic [`RuntimeBridge`] through native Vello and return runtime artifacts.
pub fn run_native_vello_runtime_with_artifacts<Bridge, Message>(
    options: NativeRunOptions,
    bridge: Bridge,
) -> NativeGenericRunReport
where
    Bridge: RuntimeBridge<Message> + 'static,
    Message: 'static,
{
    info!("radiant generic native vello: creating event loop");
    let run_started = Instant::now();
    let event_loop = match EventLoop::<RuntimeUserEvent>::with_user_event().build() {
        Ok(event_loop) => event_loop,
        Err(err) => {
            return NativeGenericRunReport {
                artifacts: NativeGenericRuntimeArtifacts::default(),
                result: Err(NativeGenericRunError::EventLoopBuild(err.to_string())),
            };
        }
    };
    let viewport = initial_viewport(&options);
    let mut runner = GenericNativeVelloRunner::new(options, bridge, viewport);
    let proxy = event_loop.create_proxy();
    let repaint_signal: Arc<dyn RepaintSignal> = runner.runtime_wakeup.install_proxy(proxy);
    runner
        .core
        .runtime
        .bridge_mut()
        .install_repaint_signal(repaint_signal);
    let run_result = event_loop
        .run_app(&mut runner)
        .map_err(|err| NativeGenericRunError::EventLoopRun(err.to_string()));
    let elapsed = run_started.elapsed();
    match &run_result {
        Ok(_) => info!(
            "radiant generic native vello: event loop ended in {} ms",
            elapsed.as_millis()
        ),
        Err(err) => warn!(
            "radiant generic native vello: event loop returned error in {} ms: {}",
            elapsed.as_millis(),
            err
        ),
    }
    let shutdown_timing = runner.core.runtime.bridge_mut().on_runtime_exit();
    NativeGenericRunReport {
        artifacts: NativeGenericRuntimeArtifacts {
            startup_timing: runner.startup_timing.export_artifact(),
            shutdown_timing,
        },
        result: run_result,
    }
}

/// Structured runtime artifacts exported after one generic native run completes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NativeGenericRuntimeArtifacts {
    /// Native startup timing artifact captured for this run, when startup began.
    pub startup_timing: Option<NativeStartupTimingArtifact>,
    /// Host-defined shutdown artifact captured after the runtime exit hook runs.
    pub shutdown_timing: Option<serde_json::Value>,
}

/// Typed failure reported by the generic native Vello runtime.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeGenericRunError {
    /// Creating the native event loop failed before runtime startup.
    EventLoopBuild(String),
    /// The native event loop returned an error while running.
    EventLoopRun(String),
}

impl std::fmt::Display for NativeGenericRunError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EventLoopBuild(message) => {
                write!(formatter, "failed to create native event loop: {message}")
            }
            Self::EventLoopRun(message) => {
                write!(formatter, "native event loop failed: {message}")
            }
        }
    }
}

impl std::error::Error for NativeGenericRunError {}

/// Result plus structured artifacts returned by one generic native runtime execution.
pub type NativeGenericRunReport =
    crate::gui_runtime::RuntimeRunReport<NativeGenericRuntimeArtifacts, NativeGenericRunError>;

fn initial_viewport(options: &NativeRunOptions) -> Vector2 {
    let [width, height] = options.inner_size.unwrap_or([1280.0, 720.0]);
    Vector2::new(width.max(1.0), height.max(1.0))
}

#[cfg(test)]
mod tests;
