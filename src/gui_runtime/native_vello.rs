//! Native `winit + vello` runtime for generic `RuntimeBridge` hosts.

use super::{
    NativeGpuBackend, NativeRunOptions, NativeRunOptionsError, NativeTextOptions, WindowIconRgba,
};
use crate::gui::{
    paint::TextAlign,
    types::{Point, Rect as UiRect, Rgba8, Vector2},
};
use crate::runtime::{PaintTextInput, RuntimeBridge};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::{info, warn};
use vello::util::RenderContext;
use vello::{Scene, wgpu};
use winit::event_loop::{ActiveEventLoop, EventLoop};

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
    generic_runtime::{
        NativeGenericRunError, NativeGenericRunReport, NativeGenericRuntimeArtifacts,
        run_native_vello_runtime, run_native_vello_runtime_with_artifacts,
    },
    startup::NativeStartupTimingArtifact,
};
