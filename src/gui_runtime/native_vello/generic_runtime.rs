//! Generic `RuntimeBridge` native Vello runner.

use super::{NativeRunOptions, RuntimeUserEvent};
use crate::{
    gui::{repaint::RepaintSignal, types::Vector2},
    runtime::RuntimeBridge,
};
use std::{sync::Arc, time::Instant};
use tracing::{info, warn};
use winit::event_loop::EventLoop;

#[cfg(test)]
use crate::{
    gui::types::{Point, Rect as UiRect, Rgba8},
    gui_runtime::native_vello::NativeTextRenderer,
};
#[cfg(test)]
use std::time::Duration;
#[cfg(test)]
use vello::Scene;

mod automation_export;
mod auxiliary;
mod composited_base;
mod core;
mod device;
mod event_routing;
mod external_drag;
mod frame_cadence;
mod frame_prepare;
mod frame_state;
mod gpu_surface;
mod gpu_surface_cursor;
mod gpu_surface_interaction;
mod gpu_surface_wheel;
mod gpu_upload_bytes;
mod input;
mod keyboard;
mod lifecycle;
mod lifecycle_pointer;
mod native_cursor;
mod native_file_drop;
mod native_file_open;
mod native_pointer;
mod pointer_click;
mod popup_drag;
mod post_gpu_overlay;
mod present;
mod render_profile;
mod route_outcome;
mod run_report;
mod runner;
mod runner_state;
mod runtime_helpers;
mod runtime_wakeup;
mod scene;
mod scene_texture;
mod surface;
mod surface_size;
mod window;

use automation_export::NativeAutomationTargetExporter;
use auxiliary::{AuxiliaryNativeWindow, AuxiliaryWindowEventResult};
use composited_base::CompositedBaseFrame;
pub(in crate::gui_runtime::native_vello) use core::{GenericNativeRuntimeCore, PointerPressStamp};
use frame_cadence::{
    TimedFrameCadence, animation_frame_interval, animation_frame_interval_for_normalized_fps,
    timed_frame_cadence, timed_frame_target_fps,
};
use frame_state::NativeVelloFrameState;
use gpu_surface::GpuSurfaceRenderer;
use gpu_surface_wheel::PendingGpuSurfaceWheel;
use gpu_surface_wheel::PendingScrollbarDrag;
use input::{
    key_code_from_winit, keypress_from_input, logical_point_from_winit, pointer_button_from_winit,
    pointer_modifiers_from_winit,
};
#[cfg(test)]
use native_pointer::{
    NativeMouseInputRoute, NativePointerEventKind, NativePointerRouteResult, NativeWheelRoute,
};
use pointer_click::pointer_press_event;
use popup_drag::should_start_popup_window_drag;
use post_gpu_overlay::PostGpuOverlayRenderer;
use render_profile::{
    RenderFrameProfile, maybe_log_render_profile, maybe_log_slow_render_profile,
    slow_render_profile_enabled,
};
pub(in crate::gui_runtime::native_vello) use route_outcome::{
    FrameWork, FrameWorkReason, GenericRouteOutcome, SceneRebuildMode,
};
pub use run_report::{
    NativeGenericRunError, NativeGenericRunReport, NativeGenericRuntimeArtifacts,
};
use runner::GenericNativeVelloRunner;
use runner_state::{NativeRunnerInputState, NativeRunnerTimingState, NativeRunnerWindowState};
pub(in crate::gui_runtime::native_vello) use runtime_helpers::GpuSurfaceInteractionRegion;
use runtime_helpers::{
    maybe_log_route_profile, render_profile_enabled,
    scroll_delta_to_logical,
};
use runtime_wakeup::RuntimeWakeup;
pub(in crate::gui_runtime::native_vello) use scene::{
    RetainedSurfaceEncodeStats, RetainedSurfaceFrameCache, SceneClipState, SceneTextRunBuffer,
    SurfaceSceneEncodeContext, encode_surface_paint_plan_to_scene,
};
use surface_size::RenderSurfacePixelSize;
use window::{
    generic_window_attributes, hide_window_after_first_present, owner_window_handle,
    reveal_window_after_first_present, reveal_window_after_surface_setup,
};

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
    if let Err(err) = options.validate() {
        return NativeGenericRunReport {
            artifacts: NativeGenericRuntimeArtifacts::default(),
            result: Err(NativeGenericRunError::InvalidWindowOptions(err)),
        };
    }
    let mut event_loop_builder = EventLoop::<RuntimeUserEvent>::with_user_event();
    let event_loop = match event_loop_builder.build() {
        Ok(event_loop) => event_loop,
        Err(err) => {
            return NativeGenericRunReport {
                artifacts: NativeGenericRuntimeArtifacts::default(),
                result: Err(NativeGenericRunError::EventLoopBuild(err.to_string())),
            };
        }
    };
    let viewport = initial_viewport(&options);
    let native_file_open_events =
        native_file_open::install_native_file_open_handler(event_loop.create_proxy());
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
    drop(native_file_open_events);
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
            startup_timing: runner.timing.startup_timing.export_artifact(),
            shutdown_timing,
        },
        result: run_result,
    }
}

fn initial_viewport(options: &NativeRunOptions) -> Vector2 {
    let [width, height] = options
        .window
        .geometry
        .inner_size
        .unwrap_or([1280.0, 720.0]);
    Vector2::new(width.max(1.0), height.max(1.0))
}

#[cfg(test)]
mod tests;
