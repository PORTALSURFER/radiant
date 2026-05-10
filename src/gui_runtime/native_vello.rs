//! Native `winit + vello` runtime for generic `RuntimeBridge` hosts.

use super::{NativeGpuBackend, NativeRunOptions, NativeTextOptions, WindowIconRgba};
use crate::gui::{
    paint::{PaintFrame, Primitive, TextAlign, TextRun},
    types::{Point, Rect as UiRect, Rgba8, Vector2},
};
use crate::runtime::{PaintPrimitive, PaintTextAlign, PaintTextInput, RuntimeBridge};
use crate::widgets::{RetainedSurfaceDescriptor, TextEditCommand, WidgetKey};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::{error, info, warn};
use vello::util::{RenderContext, RenderSurface};
use vello::{
    AaConfig, AaSupport, RenderParams, Renderer, RendererOptions, Scene,
    kurbo::{Affine, BezPath, Circle, Point as KurboPoint},
    peniko::{Blob, Fill, Gradient, ImageAlphaType, ImageData, ImageFormat},
    wgpu,
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, Size},
    event::{ElementState, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

mod generic_runtime;
mod runtime_config;
mod runtime_event;
#[allow(dead_code)]
mod startup;
#[allow(dead_code)]
mod text_edit;
#[allow(dead_code)]
mod text_renderer;

use self::{startup::*, text_renderer::*};
pub(in crate::gui_runtime::native_vello) use runtime_config::*;
pub(in crate::gui_runtime::native_vello) use runtime_event::RuntimeUserEvent;

pub use self::{
    generic_runtime::{
        NativeGenericRunReport, NativeGenericRuntimeArtifacts, run_native_vello_runtime,
        run_native_vello_runtime_with_artifacts,
    },
    startup::NativeStartupTimingArtifact,
};
