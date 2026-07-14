//! Native `winit + vello` runtime for generic `RuntimeBridge` hosts.

use super::{NativeRunOptions, NativeTextOptions, WindowIconRgba};
use crate::gui::{
    paint::TextAlign,
    types::{Point, Rect as UiRect, Rgba8},
};
use crate::runtime::PaintTextInput;
use vello::{Scene, wgpu};

mod embedded;
mod generic_runtime;
mod runtime_config;
mod runtime_event;
mod startup;
mod text_edit;
mod text_renderer;

use self::text_renderer::*;
pub(in crate::gui_runtime::native_vello) use runtime_config::{
    select_present_mode, startup_renderer_options,
};
pub(in crate::gui_runtime::native_vello) use runtime_event::RuntimeUserEvent;

pub use self::{
    embedded::{
        EmbeddedVelloError, EmbeddedVelloRenderer, EmbeddedVelloSurfaceHandle,
        EmbeddedVelloUnsupportedPrimitive,
    },
    generic_runtime::{
        GpuSurfaceOcclusionPlanningScratch, NativeGenericRunError, NativeGenericRunReport,
        NativeGenericRuntimeArtifacts, plan_gpu_surface_occlusion_for_diagnostics,
        run_native_vello_runtime, run_native_vello_runtime_with_artifacts,
    },
    startup::NativeStartupTimingArtifact,
};
